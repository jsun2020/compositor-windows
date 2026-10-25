// Keep real production UI assertions unchanged; bound only binary POST copies in CDP events.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {chromium}=require('@playwright/test');
const {startProxy}=require('./mac157-cdp-bounded-post.cjs');
function run(protocol){
 assert(['verify-mac157-regression-native.cjs','verify-mac157-native.cjs'].includes(protocol));
 const out=process.argv[3];assert(out);
 const log=(type,data={})=>fs.appendFileSync(path.join(out,'cdp-transport.jsonl'),JSON.stringify({utc:new Date().toISOString(),type,...data})+'\n');
 const connect=chromium.connectOverCDP.bind(chromium);
 chromium.connectOverCDP=async(endpoint,options)=>{
  const proxy=await startProxy(endpoint,log);
  try{const browser=await connect(proxy.endpoint,options);browser.once('disconnected',()=>{proxy.close().catch(e=>log('proxy-close-error',{message:String(e)}))});return browser;}
  catch(e){await proxy.close();throw e;}
 };
 log('protocol',{protocol,scope:'Original production UI protocol; only CDP POST notification copies bounded'});
 require('./'+protocol);
}
module.exports={run};
