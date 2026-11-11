const {runInNewContext} = require('node:vm');
const {readFileSync} = require('node:fs');
const assert = require('node:assert/strict');
let poll, message, ad=false, toggles=0, sought;
const video={currentTime:7,duration:120,paused:true,ended:false,readyState:4,muted:false};
const player={playVideo(){toggles++},pauseVideo(){toggles--},getPlayerState(){return 2},seekTo(t){sought=t}};
const window={ipc:{postMessage(value){message=JSON.parse(value)}}};window.top=window;
runInNewContext(readFileSync(require('node:path').join(__dirname,'../../assets/preview.js'),'utf8'),{
 window, setInterval(callback){poll=callback}, document:{getElementById(){return player},querySelector(selector){return selector==='video'?video:selector==='.ad-showing'&&ad?{}:null}}
});
poll();assert.equal(message.time,7);assert.equal(message.duration,120);
ad=true;video.currentTime=3;video.duration=30;poll();assert.equal(message.time,7);assert.equal(message.duration,120);assert.equal(message.ready,false);
window.flowmoorPreview.seek(42);assert.equal(sought,undefined);
ad=false;window.flowmoorPreview.seek(42);assert.equal(sought,42);
window.flowmoorPreview.toggle();assert.equal(toggles,1);
console.log('Preview: first play, seek and ad clock isolation passed');

let twitchClicks=0;
const twitchWindow={ipc:{postMessage(){}}};twitchWindow.top=twitchWindow;
runInNewContext(readFileSync(require('node:path').join(__dirname,'../../assets/preview.js'),'utf8'),{
 window:twitchWindow,setInterval(){},document:{getElementById(){return null},querySelector(s){return s.includes('player-play-pause-button')?{click(){twitchClicks++}}:null}}
});
twitchWindow.flowmoorPreview.toggle();assert.equal(twitchClicks,1);

let failurePoll, failureMessage;
const directWindow={ipc:{postMessage(value){failureMessage=JSON.parse(value)}}};directWindow.top=directWindow;
const directVideo={currentTime:0,duration:120,paused:true,readyState:0,play(){return {catch(callback){callback(new Error('Playback denied'))}}}};
runInNewContext(readFileSync(require('node:path').join(__dirname,'../../assets/preview.js'),'utf8'),{
 window:directWindow,setInterval(callback){failurePoll=callback},document:{getElementById(){return null},querySelector(selector){return selector==='video'?directVideo:null}}
});
directWindow.flowmoorPreview.toggle();failurePoll();assert.equal(failureMessage.error,'Playback denied');
console.log('Preview: direct stream playback failure is surfaced');
