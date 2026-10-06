// Supplemental production WebView2/IPC checks. CDP input and a synthetic Tauri
// drop event exercise the real file bridge; they do not establish OS mouse input.
const {chromium,expect}=require('@playwright/test');
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),crypto=require('node:crypto');
const [marker,out,port,probes]=process.argv.slice(2);let browser,page;
const hash=b=>crypto.createHash('sha256').update(b).digest('hex');
const rows=[];
(async()=>{
  assert(marker&&out&&port&&probes,'marker, fresh evidence directory, port and probes required');
  const receipt=JSON.parse(fs.readFileSync(path.join(probes,'input-receipt.json'),'utf8').replace(/^\uFEFF/,''));
  const frozen=receipt.files.filter(f=>f.name!=='RESULT.txt');
  const checkInputs=()=>{for(const file of frozen){const bytes=fs.readFileSync(path.join(probes,file.name));assert.equal(bytes.length,file.bytes,file.name);assert.equal(hash(bytes),file.sha256.toLowerCase(),file.name);}};checkInputs();
  browser=await chromium.connectOverCDP(`http://127.0.0.1:${port}`,{timeout:30000});
  page=browser.contexts()[0].pages().find(p=>p.url().includes('tauri.localhost'));
  assert(page,'Production native page required');
  await expect(page.getByTestId('engine-ready')).toContainText(marker,{timeout:30000});
  const identity=await page.evaluate(()=>({testApi:typeof window.__compositor,native:'__TAURI_INTERNALS__' in window}));
  assert.equal(identity.testApi,'undefined');assert.equal(identity.native,true);
  const errors=[],warnings=[];page.on('pageerror',e=>errors.push(e.message));
  page.on('dialog',async d=>{warnings.push(d.message());await d.accept();});
  const drop=async file=>{await page.evaluate(file=>window.__TAURI_INTERNALS__.invoke('plugin:event|emit_to',{target:{kind:'Webview',label:'main'},event:'tauri://drag-drop',payload:{paths:[file],position:{x:0,y:0}}}),path.resolve(file));};
  const fresh=async()=>{await page.reload();await expect(page.getByTestId('engine-ready')).toContainText(marker);await expect(page.getByTestId('project-tab')).toHaveCount(0);};
  const menu=async(name,id)=>{await page.getByRole('button',{name,exact:true}).click();await page.getByTestId('menu-'+id).click();};
  const picture=async()=>{await page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));return page.getByTestId('canvas-view').locator('canvas').first().evaluate(c=>c.toDataURL('image/png'));};
  const readable=async dialog=>{const style=await dialog.locator('.sheet').evaluate(el=>{const s=getComputedStyle(el);return {color:s.color,background:s.backgroundColor,font:s.fontFamily};});const luminance=value=>{const channels=value.match(/[\d.]+/g).slice(0,3).map(Number).map(v=>{v/=255;return v<=.04045?v/12.92:((v+.055)/1.055)**2.4;});return channels[0]*.2126+channels[1]*.7152+channels[2]*.0722;};const a=luminance(style.color),b=luminance(style.background),contrast=(Math.max(a,b)+.05)/(Math.min(a,b)+.05);assert(contrast>=4.5,`Dialog text contrast ${contrast}: ${JSON.stringify(style)}`);assert(/sans-serif/i.test(style.font),'Dialog must inherit the application font outside its root');return {contrast,...style};};
  const number=async(name,value)=>{const input=page.getByRole('spinbutton',{name,exact:true});await input.fill(String(value));await input.press('Enter');};
  const dng=path.resolve(probes,'bayer-no-preview.dng');const original=hash(fs.readFileSync(dng));
  const info=await page.evaluate(async file=>window.__TAURI_INTERNALS__.invoke('raw_inspect',{path:file,token:crypto.randomUUID()}),dng);
  assert.equal(info.sensorDecoded,false);assert.deepEqual([info.width,info.height],[128,96]);
  const settings={exposure:0,temperature:info.asShotTemperature,tint:info.asShotTint,tone:1};
  const develop=async controls=>Buffer.from(await page.evaluate(async({file,settings})=>Array.from(new Uint8Array(await window.__TAURI_INTERNALS__.invoke('raw_develop',{path:file,settings,preview:false,token:crypto.randomUUID()}))),{file:dng,settings:controls}));
  const normal=await develop(settings),again=await develop(settings),bright=await develop({...settings,exposure:1});
  assert(normal.equals(again),'Native IPC As Shot must repeat exactly');assert(!normal.equals(bright),'Native IPC exposure must change pixels');
  fs.writeFileSync(path.join(out,'raw-native-as-shot.png'),normal,{flag:'wx'});fs.writeFileSync(path.join(out,'raw-native-exposure-plus1.png'),bright,{flag:'wx'});
  const cancelled=await page.evaluate(async({file,settings})=>{const token=crypto.randomUUID();await window.__TAURI_INTERNALS__.invoke('raw_cancel',{token});try{await window.__TAURI_INTERNALS__.invoke('raw_develop',{path:file,settings,preview:false,token});return null;}catch(e){return String(e);}},{file:dng,settings});
  assert(cancelled&&/cancelled/.test(cancelled),'Native pre-cancel must be retained');
  const invalid=await page.evaluate(async({file,settings})=>{try{await window.__TAURI_INTERNALS__.invoke('raw_develop',{path:file,settings:{...settings,temperature:0},preview:false,token:crypto.randomUUID()});return null;}catch(e){return String(e);}},{file:dng,settings});assert(invalid&&/Invalid RAW/.test(invalid),`Invalid settings response: ${invalid}`);
  rows.push({case:'native RAW IPC',info,asShotSHA256:hash(normal),exposureSHA256:hash(bright),preCancel:cancelled,invalidSettings:invalid});
  // The real RAW dialog, guard and successful full-size import, through production routing.
  await drop(dng);await expect(page.getByRole('dialog',{name:'Develop Camera RAW',exact:true})).toBeVisible();await expect(page.getByTestId('raw-preview')).toBeVisible({timeout:30000});await readable(page.getByRole('dialog',{name:'Develop Camera RAW',exact:true}));
  await number('Exposure',1);await expect(page.getByRole('dialog',{name:'Develop Camera RAW',exact:true})).toBeVisible();await page.getByRole('button',{name:'Cancel',exact:true}).click();await expect(page.getByTestId('project-tab')).toHaveCount(0);
  await drop(dng);await expect(page.getByTestId('raw-preview')).toBeVisible({timeout:30000});await number('Exposure',1);await page.getByRole('button',{name:'Reset to As Shot',exact:true}).click();await expect(page.getByRole('spinbutton',{name:'Exposure',exact:true})).toHaveValue('0');
  await page.getByRole('button',{name:'Import',exact:true}).click();await expect(page.getByRole('dialog',{name:'Develop Camera RAW',exact:true})).toHaveCount(0,{timeout:30000});await expect(page.getByTestId('project-tab')).toHaveCount(1);await expect(page.getByTestId('layer-row')).toHaveCount(1);await page.screenshot({path:path.join(out,'raw-imported-window.png')});rows.push({case:'production RAW dialog',preview:true,fieldEnterKeepsDialog:true,cancelNoDocument:true,asShotReset:true,fullImport:true});
  // Worker grade, preview toggle, Cancel, one-step Undo/Redo and preview-only diagnostics.
  await fresh();await drop(path.join(probes,'CameraRaw','01-light-color-curve-source.comp'));await expect(page.getByTestId('project-tab')).toHaveCount(1);const source=await picture();
  await menu('Filter','filter-camera-raw');await number('Exposure',1);await expect.poll(picture).not.toEqual(source);await page.getByTestId('adjust-preview').uncheck();await expect.poll(picture).toEqual(source);await page.getByTestId('adjust-cancel').click();await expect.poll(picture).toEqual(source);
  await menu('Filter','filter-camera-raw');await number('Exposure',1);await page.getByTestId('adjust-ok').click();await expect(page.getByTestId('adjust-panel')).toHaveCount(0);await expect.poll(picture).not.toEqual(source);const applied=await picture();await page.keyboard.press('Control+z');await expect.poll(picture).toEqual(source);await page.keyboard.press('Control+Shift+z');await expect.poll(picture).toEqual(applied);
  await menu('Filter','filter-camera-raw');await page.getByRole('combobox',{name:'Clipping view',exact:true}).selectOption('1');await expect.poll(picture).not.toEqual(applied);await page.getByTestId('adjust-ok').click();await expect.poll(picture).toEqual(applied);await page.screenshot({path:path.join(out,'camera-raw-applied-window.png')});rows.push({case:'production Camera Raw',previewChanges:true,previewOffAndCancelRestore:true,oneUndoRedo:true,diagnosticsNotBaked:true});
  const fixtures=[['01-groups-masks-clipping.psd',['Folder','Background']],['01-groups-masks-clipping.psb',['Folder','Background']],['02-editable-text-shape.psd',['Editable text','Editable rectangle']],['03-adjustment-conversions.psd',['Levels']],['../Real-PSD/source.psd',['图层 1','背景']]];
  for(const [name,names] of fixtures){await fresh();const before=warnings.length;await drop(path.resolve(probes,'Photoshop',name));await expect(page.getByTestId('project-tab')).toHaveCount(1);for(const name of names)await expect(page.getByTestId('layer-row').filter({hasText:name}).first()).toBeVisible();const report=page.getByRole('dialog',{name:'Photoshop import conversions',exact:true});if(name.includes('editable')||name.includes('adjustment')){await expect(report).toBeVisible();await readable(report);const text=await report.innerText();warnings.push(text);if(name.includes('adjustment'))assert(/unsupported.*skipped/is.test(text));await page.screenshot({path:path.join(out,path.basename(name).replaceAll('.','-')+'-report.png')});await report.getByRole('button',{name:'Close',exact:true}).click();await expect(report).toHaveCount(0);}await page.screenshot({path:path.join(out,path.basename(name).replaceAll('.','-')+'-window.png')});rows.push({case:'production Photoshop import',input:name,visibleNames:names,warnings:warnings.slice(before)});}
  // Closing an imported, unsaved project must expose all three choices. Only
  // the save picker's response is supplied below; package read/write IPC is native.
  const closeDialog=()=>page.getByRole('dialog',{name:'Close project',exact:true});
  const requestClose=async()=>{await page.getByTestId('project-tab').last().getByRole('button').click();await expect(closeDialog()).toBeVisible();};
  const setSavePick=async target=>page.evaluate(target=>{
    const native=window.fetch.bind(window);
    window.fetch=(request,options)=>{
      const url=typeof request==='string'?request:request.url;
      if(decodeURIComponent(new URL(url,location.href).pathname)==='/plugin:dialog|save')
        return Promise.resolve(new Response(JSON.stringify(target),{headers:{'Tauri-Response':'ok','Content-Type':'application/json'}}));
      return native(request,options);
    };
  },target);
  await requestClose();await readable(closeDialog());
  await expect(closeDialog().getByRole('button',{name:'Cancel Close',exact:true})).toBeFocused();
  await page.screenshot({path:path.join(out,'close-project-choices.png')});
  await page.keyboard.press('Escape');await expect(closeDialog()).toHaveCount(0);await expect(page.getByTestId('project-tab')).toHaveCount(1);
  await requestClose();await closeDialog().getByRole('button',{name:'Cancel Close',exact:true}).click();await expect(closeDialog()).toHaveCount(0);await expect(page.getByTestId('project-tab')).toContainText('•');
  await setSavePick(null);await requestClose();await closeDialog().getByRole('button',{name:'Save and Close',exact:true}).click();await expect(closeDialog()).toHaveCount(0);await expect(page.getByTestId('working')).toHaveCount(0);await expect(page.getByTestId('project-tab')).toContainText('•');
  await requestClose();await closeDialog().getByRole('button',{name:"Don't Save and Close",exact:true}).click();await expect(page.getByTestId('project-tab')).toHaveCount(0);await expect(page.getByTestId('layer-row')).toHaveCount(0);await expect(page.getByText('Open an image or create a new canvas',{exact:true})).toBeVisible();
  const cleared=await picture();
  const empty=await page.evaluate(async src=>{const img=new Image();img.src=src;await img.decode();const canvas=document.createElement('canvas');canvas.width=img.width;canvas.height=img.height;const ctx=canvas.getContext('2d');ctx.drawImage(img,0,0);return ctx.getImageData(0,0,canvas.width,canvas.height).data.every((v,i)=>Math.abs(v-(i%4===3?255:41))<=1);},cleared);assert(empty,'Discard close must clear the actual canvas');
  await fresh();await drop(path.resolve(probes,'Real-PSD','source.psd'));await expect(page.getByTestId('layer-row')).toHaveCount(2);
  const saved=path.join(out,'close-saved.comp');assert(!fs.existsSync(saved));await setSavePick(saved);await requestClose();await closeDialog().getByRole('button',{name:'Save and Close',exact:true}).click();await expect(page.getByTestId('project-tab')).toHaveCount(0,{timeout:30000});assert(fs.existsSync(path.join(saved,'manifest.json')),'Native save must complete before close');
  await drop(saved);await expect(page.getByTestId('layer-row')).toHaveCount(2);await expect(page.getByTestId('project-tab')).not.toContainText('•');rows.push({case:'production close project',explicitChoices:true,cancelAndEscapeRetainChanges:true,cancelledSaveAsRetainsChanges:true,discardClosesAndClearsCanvas:true,nativeSaveThenCloseAndReopen:true});
  assert.equal(hash(fs.readFileSync(dng)),original,'Original DNG changed');checkInputs();assert.deepEqual(errors,[]);
  fs.writeFileSync(path.join(out,'result.json'),JSON.stringify({status:'PASS',marker,identity,rows,pageErrors:errors,scope:'Production native WebView2, real file/RAW/package IPC, synthetic Tauri drop and CDP menu/keyboard input; only the save-picker HTTP response is supplied. Actual OS file-picker/mouse and real-camera RAW remain separate.'},null,2),{flag:'wx'});
  console.log('PASS: production RAW, Camera Raw, five Photoshop imports and close/cancel/discard/native save/reopen.');await browser.close();
})().then(()=>process.exit(0)).catch(async e=>{console.error(e);fs.writeFileSync(path.join(out,'failure-result.json'),JSON.stringify({status:'FAIL',error:String(e),stack:e.stack,completed:rows},null,2),{flag:'wx'});if(page){await page.screenshot({path:path.join(out,'failure-window.png')}).catch(()=>{});fs.writeFileSync(path.join(out,'failure-dom.txt'),await page.locator('body').innerText().catch(String));}process.exit(1);});
