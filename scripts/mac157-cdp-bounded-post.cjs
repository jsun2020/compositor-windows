// Diagnostic transport: cap only the POST data copied into CDP notifications.
// Application requests, responses, UI commands and assertions pass unchanged.
const path=require('node:path'),assert=require('node:assert/strict');
const fromTest=require('node:module').createRequire(require.resolve('@playwright/test'));
const fromPlaywright=require('node:module').createRequire(fromTest.resolve('playwright'));
const {ws:WS,wsServer:WSServer}=require(path.join(path.dirname(fromPlaywright.resolve('playwright-core/package.json')),'lib/utilsBundle.js'));
async function startProxy(endpoint,log=()=>{}){
 const url=new URL(endpoint);assert(['127.0.0.1','localhost'].includes(url.hostname),'Loopback only');
 const upstream=/^ws/.test(url.protocol)?endpoint:(await (await fetch(new URL('/json/version',url),{signal:AbortSignal.timeout(10000)})).json()).webSocketDebuggerUrl;
 assert(['127.0.0.1','localhost'].includes(new URL(upstream).hostname));
 const server=new WSServer({host:'127.0.0.1',port:0,maxPayload:256*1024*1024,perMessageDeflate:false});
 await new Promise((resolve,reject)=>{server.once('listening',resolve);server.once('error',reject)});
 const sockets=new Set();
 server.on('connection',client=>{
  const remote=new WS(upstream,{maxPayload:256*1024*1024,perMessageDeflate:false});sockets.add(client);sockets.add(remote);let queue=[];
  const abort=(side,e)=>{log('cdp-transport-error',{side,message:String(e)});client.terminate();remote.terminate();};
  client.on('error',e=>abort('client',e));remote.on('error',e=>abort('upstream',e));
  client.on('close',()=>{sockets.delete(client);remote.close()});remote.on('close',(code,reason)=>{log('cdp-upstream-close',{code,reason:reason.toString()});sockets.delete(remote);client.close()});
  remote.on('open',()=>{for(const [data,binary] of queue)remote.send(data,{binary});queue=[]});
  client.on('message',(data,binary)=>{
   try{
    const command=JSON.parse(data.toString());
    if(command.method==='Network.enable'){
     command.params={...command.params,maxPostDataSize:65536};data=Buffer.from(JSON.stringify(command));binary=false;
     log('cdp-post-data-cap',{sessionId:command.sessionId??null,maxPostDataSize:65536});
    }
    if(remote.readyState===WS.OPEN)remote.send(data,{binary});else if(remote.readyState===WS.CONNECTING)queue.push([data,binary]);
   }catch(e){abort('parse-command',e)}
  });
  remote.on('message',(data,binary)=>{if(data.length>1024*1024)log('cdp-large-message',{bytes:data.length});if(client.readyState===WS.OPEN)client.send(data,{binary})});
 });
 return {endpoint:`ws://127.0.0.1:${server.address().port}`,close:async()=>{for(const socket of sockets)socket.terminate();await new Promise(r=>server.close(r))}};
}
module.exports={startProxy};
