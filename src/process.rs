use std::{ffi::OsStr, io, process::Command};

/// Noninteractive tools must not open a console in the desktop application.
pub(crate) fn command(program: impl AsRef<OsStr>) -> Command {
    #[allow(unused_mut)]
    let mut command = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(windows_sys::Win32::System::Threading::CREATE_NO_WINDOW);
    }
    command
}

#[cfg(unix)]
pub(crate) fn spawn(command: &mut Command) -> io::Result<std::process::Child> {
    use std::{os::unix::process::CommandExt, process::Stdio};
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)
        .spawn()
}

#[cfg(unix)]
pub(crate) fn signal(child: &mut std::process::Child, graceful: bool) -> io::Result<()> {
    let result = unsafe {
        libc::kill(
            -(child.id() as i32),
            if graceful {
                libc::SIGINT
            } else {
                libc::SIGKILL
            },
        )
    };
    let error = io::Error::last_os_error();
    if result == 0 || child.try_wait()?.is_some() {
        Ok(())
    } else {
        Err(error)
    }
}

/// Entry-point dispatch for the short-lived Windows Ctrl+C relay.
/// A separate process keeps the application's console and concurrent jobs untouched.
pub fn handle_signal() {
    #[cfg(windows)]
    if std::env::args_os().nth(1).as_deref() == Some(OsStr::new("--flowmoor-process-signal")) {
        use windows_sys::Win32::System::Console::*;
        let pid = std::env::args()
            .nth(2)
            .and_then(|value| value.parse::<u32>().ok());
        let success = pid.is_some_and(|pid| unsafe {
            if AttachConsole(pid) == 0 {
                return false;
            }
            let sent = SetConsoleCtrlHandler(None, 1) != 0
                && GenerateConsoleCtrlEvent(CTRL_C_EVENT, 0) != 0;
            // Console control handlers run asynchronously.
            std::thread::sleep(std::time::Duration::from_millis(100));
            FreeConsole();
            sent
        });
        std::process::exit(if success { 0 } else { 1 });
    }
}

#[cfg(windows)]
pub(crate) use windows::{signal, spawn};

#[cfg(windows)]
mod windows {
    use super::*;
    use std::{
        mem::{size_of, size_of_val},
        os::windows::{
            ffi::OsStrExt,
            io::{AsRawHandle, FromRawHandle, OwnedHandle},
            process::ExitStatusExt,
        },
        process::ExitStatus,
        ptr::{null, null_mut},
    };
    use windows_sys::Win32::{
        Foundation::*,
        System::{JobObjects::*, Threading::*},
        UI::WindowsAndMessaging::SW_HIDE,
    };

    pub(crate) struct Child {
        process: OwnedHandle,
        job: OwnedHandle,
        pid: u32,
        pub stdout: Option<io::PipeReader>,
        pub stderr: Option<io::PipeReader>,
    }
    impl Child {
        pub fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
            match unsafe { WaitForSingleObject(self.process.as_raw_handle(), 0) } {
                WAIT_TIMEOUT => Ok(None),
                WAIT_OBJECT_0 => {
                    let mut code = 0;
                    if unsafe { GetExitCodeProcess(self.process.as_raw_handle(), &mut code) } == 0 {
                        return Err(io::Error::last_os_error());
                    }
                    Ok(Some(ExitStatus::from_raw(code)))
                }
                _ => Err(io::Error::last_os_error()),
            }
        }
        pub fn wait(&mut self) -> io::Result<ExitStatus> {
            if unsafe { WaitForSingleObject(self.process.as_raw_handle(), INFINITE) }
                != WAIT_OBJECT_0
            {
                return Err(io::Error::last_os_error());
            }
            self.try_wait()?
                .ok_or_else(|| io::Error::other("Process did not exit"))
        }
    }

    pub(crate) fn spawn(command: &mut Command) -> io::Result<Child> {
        // Managed tools inherit our environment and working directory; only their argv varies.
        debug_assert!(command.get_envs().next().is_none() && command.get_current_dir().is_none());
        let mut line = Vec::<u16>::new();
        for argument in std::iter::once(command.get_program()).chain(command.get_args()) {
            if !line.is_empty() {
                line.push(b' ' as u16);
            }
            line.push(b'"' as u16);
            let mut slashes = 0;
            for unit in argument.encode_wide() {
                if unit == 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "NUL in process argument",
                    ));
                }
                if unit == b'\\' as u16 {
                    slashes += 1;
                    continue;
                }
                line.extend(std::iter::repeat_n(
                    b'\\' as u16,
                    slashes * if unit == b'"' as u16 { 2 } else { 1 },
                ));
                slashes = 0;
                if unit == b'"' as u16 {
                    line.push(b'\\' as u16);
                }
                line.push(unit);
            }
            line.extend(std::iter::repeat_n(b'\\' as u16, slashes * 2));
            line.push(b'"' as u16);
        }
        line.push(0);
        let (stdout, out_write) = io::pipe()?;
        let (stderr, err_write) = io::pipe()?;
        let stdin = std::fs::File::open("NUL")?;
        let mut inherited = Vec::new();
        for handle in [
            stdin.as_raw_handle(),
            out_write.as_raw_handle(),
            err_write.as_raw_handle(),
        ] {
            let mut copy = null_mut();
            if unsafe {
                DuplicateHandle(
                    GetCurrentProcess(),
                    handle,
                    GetCurrentProcess(),
                    &mut copy,
                    0,
                    1,
                    DUPLICATE_SAME_ACCESS,
                )
            } == 0
            {
                return Err(io::Error::last_os_error());
            }
            inherited.push(unsafe { OwnedHandle::from_raw_handle(copy) });
        }
        let handles: Vec<_> = inherited.iter().map(AsRawHandle::as_raw_handle).collect();
        let mut bytes = 0;
        unsafe {
            InitializeProcThreadAttributeList(null_mut(), 1, 0, &mut bytes);
        }
        let mut storage = vec![0usize; bytes.div_ceil(size_of::<usize>())];
        let attributes = storage.as_mut_ptr().cast();
        if unsafe { InitializeProcThreadAttributeList(attributes, 1, 0, &mut bytes) } == 0 {
            return Err(io::Error::last_os_error());
        }
        let mut startup = STARTUPINFOEXW::default();
        startup.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
        startup.StartupInfo.dwFlags = STARTF_USESHOWWINDOW | STARTF_USESTDHANDLES;
        startup.StartupInfo.wShowWindow = SW_HIDE as u16;
        startup.StartupInfo.hStdInput = handles[0];
        startup.StartupInfo.hStdOutput = handles[1];
        startup.StartupInfo.hStdError = handles[2];
        startup.lpAttributeList = attributes;
        let mut info = PROCESS_INFORMATION::default();
        // Restrict inheritance to this job's pipes, so parallel downloads cannot hold them open.
        let created = unsafe {
            let updated = UpdateProcThreadAttribute(
                attributes,
                0,
                PROC_THREAD_ATTRIBUTE_HANDLE_LIST as usize,
                handles.as_ptr().cast(),
                size_of_val(handles.as_slice()),
                null_mut(),
                null(),
            ) != 0;
            let created = updated
                && CreateProcessW(
                    null(),
                    line.as_mut_ptr(),
                    null(),
                    null(),
                    1,
                    CREATE_NEW_CONSOLE | CREATE_SUSPENDED | EXTENDED_STARTUPINFO_PRESENT,
                    null(),
                    null(),
                    &startup.StartupInfo,
                    &mut info,
                ) != 0;
            let result = if created {
                Ok(())
            } else {
                Err(io::Error::last_os_error())
            };
            DeleteProcThreadAttributeList(attributes);
            result
        };
        created?;
        let process = unsafe { OwnedHandle::from_raw_handle(info.hProcess) };
        let thread = unsafe { OwnedHandle::from_raw_handle(info.hThread) };
        let job = unsafe { CreateJobObjectW(null(), null()) };
        let setup = (|| {
            if job.is_null() {
                return Err(io::Error::last_os_error());
            }
            let job = unsafe { OwnedHandle::from_raw_handle(job) };
            let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            if unsafe {
                SetInformationJobObject(
                    job.as_raw_handle(),
                    JobObjectExtendedLimitInformation,
                    (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                    size_of_val(&limits) as u32,
                )
            } == 0
                || unsafe { AssignProcessToJobObject(job.as_raw_handle(), process.as_raw_handle()) }
                    == 0
                || unsafe { ResumeThread(thread.as_raw_handle()) } == u32::MAX
            {
                return Err(io::Error::last_os_error());
            }
            Ok(job)
        })();
        match setup {
            Ok(job) => Ok(Child {
                process,
                job,
                pid: info.dwProcessId,
                stdout: Some(stdout),
                stderr: Some(stderr),
            }),
            Err(error) => {
                unsafe {
                    TerminateProcess(process.as_raw_handle(), 1);
                    WaitForSingleObject(process.as_raw_handle(), INFINITE);
                }
                Err(error)
            }
        }
    }

    pub(crate) fn signal(child: &mut Child, graceful: bool) -> io::Result<()> {
        if child.try_wait()?.is_some() {
            return Ok(());
        }
        if !graceful {
            return if unsafe { TerminateJobObject(child.job.as_raw_handle(), 1) } != 0 {
                Ok(())
            } else {
                Err(io::Error::last_os_error())
            };
        }
        let output = super::command(std::env::current_exe()?)
            .args(["--flowmoor-process-signal", &child.pid.to_string()])
            .output()?;
        if output.status.success() || child.try_wait()?.is_some() {
            Ok(())
        } else {
            Err(io::Error::other(
                "Could not send Ctrl+C to the download process",
            ))
        }
    }
}
