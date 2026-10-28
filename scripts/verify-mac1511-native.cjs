// Mac 1.5.11 bounded native group: production WebView2 + PID-scoped native dialogs.
// Separate from mac157 native/regression. No mocked bridge / no window.__compositor.
// Locators grounded in app/src (MenuBar, FilterPanel, LensEffectsPanel, CropOptions, MoveOptions, ToolRail, LayersList).
// Individual expect budgets remain 30000ms; whole group intended <=8min / one attempt.
const {chromium,expect:baseExpect}=require('@playwright/test');
const expect=baseExpect.configure({timeout:30000});
const {spawnSync}=require('node:child_process');
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),crypto=require('node:crypto');
const [marker,out,port,pid,exe,probes,rawSource]=process.argv.slice(2);
let browser,page,dialogIndex=0,pictureIndex=0;
const rows=[],errors=[];
const json=(name,value)=>fs.writeFileSync(path.join(out,name),JSON.stringify(value,null,2),{flag:'wx'});
function nativeDialog(action,target){
  const receipt=path.join(out,'dialog-'+String(++dialogIndex).padStart(2,'0')+'-'+action+'.json');
  const args=['-X','utf8',path.join(__dirname,'native-dialog-control.py'),pid,exe,action,'--output',receipt];
  if(target)args.push('--path',target);
  const result=spawnSync('python.exe',args,{encoding:'utf8',timeout:45000});
  console.log(result.stdout.trim());if(result.stderr)console.error(result.stderr.trim());
  assert.equal(result.status,0,'Native '+action+' failed: '+receipt);
  return JSON.parse(fs.readFileSync(receipt,'utf8'));
}
async function menu(title,id){
  await expect(page.getByTestId('working')).toHaveCount(0,{timeout:30000});
  await page.getByTestId('menubar-'+title.toLowerCase()).click();
  await page.getByTestId('menu-'+id).click();
}
async function picture(){
  await page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));
  const image=await page.getByTestId('canvas-view').locator('canvas').first().evaluate(async c=>{
    const gl=c.getContext('webgl2'),rgba=new Uint8Array(c.width*c.height*4);
    gl.readPixels(0,0,c.width,c.height,gl.RGBA,gl.UNSIGNED_BYTE,rgba);
    const digest=await crypto.subtle.digest('SHA-256',rgba);
    return {pixels:c.width+'x'+c.height+':'+Array.from(new Uint8Array(digest),n=>n.toString(16).padStart(2,'0')).join(''),png:c.toDataURL('image/png')};
  });
  fs.writeFileSync(path.join(out,'view-'+String(++pictureIndex).padStart(3,'0')+'.png'),Buffer.from(image.png.split(',')[1],'base64'));
  return image.pixels;
}
async function capture(name){
  await page.screenshot({path:path.join(out,name+'.png')});
  fs.writeFileSync(path.join(out,name+'.txt'),await page.locator('body').innerText(),{flag:'wx'});
}
async function save(name){
  const target=path.join(out,name+'.comp');
  await menu('File','save-as');
  nativeDialog('select',target);
  await expect.poll(async()=>fs.existsSync(path.join(target,'manifest.json'))||(await page.getByTestId('error-banner').count())>0,{timeout:30000}).toBe(true);
  await expect(page.getByTestId('error-banner')).toHaveCount(0);
  await expect(page.getByTestId('working')).toHaveCount(0,{timeout:30000});
  await expect(page.getByTestId('project-tab')).not.toContainText('•');
  assert(fs.existsSync(path.join(target,'manifest.json')));
  return JSON.parse(fs.readFileSync(path.join(target,'manifest.json'),'utf8'));
}
async function close(){
  await menu('File','close');
  const sheet=page.getByRole('dialog',{name:'Close project',exact:true});
  if(await sheet.isVisible())await sheet.getByRole('button',{name:"Don't Save and Close",exact:true}).click();
  await expect(page.getByTestId('project-tab')).toHaveCount(0);
}
async function field(name,value){
  const input=page.getByTestId('adjust-panel').getByLabel(name,{exact:true});
  await input.fill(String(value));
  await input.press('Enter');
}
async function importProbe(name){
  await menu('File','import');
  nativeDialog('select',path.join(probes,name));
  await expect(page.getByTestId('working')).toHaveCount(0,{timeout:30000});
  await expect(page.getByTestId('error-banner')).toHaveCount(0);
}
async function newBlankCanvas(){
  // Transparent new canvas contains a genuinely pixel-less layer.
  await menu('File','new');
  await page.getByLabel('Units',{exact:true}).selectOption('px');
  await page.getByLabel('Width',{exact:true}).fill('240');
  await page.getByLabel('Height',{exact:true}).fill('180');
  await page.getByLabel('Background',{exact:true}).selectOption('Transparent');
  await page.getByRole('button',{name:'Create',exact:true}).click();
  await expect(page.getByTestId('layer-row')).toHaveCount(1,{timeout:30000});
}
async function openFilter(id){
  await menu('Filter',id);
  await expect(page.getByTestId('adjust-panel')).toBeVisible({timeout:30000});
}
async function selectLayerRows(indices){
  const rows=page.getByTestId('layer-row');
  for(let i=0;i<indices.length;i++){
    const row=rows.nth(indices[i]);
    if(i>0)await row.locator('.layer-name').click({modifiers:['Control']});
    else await row.locator('.layer-name').click();
  }
  await expect(page.locator('[data-testid=layer-row][aria-selected=true]')).toHaveCount(indices.length);
}

(async()=>{
  assert(marker&&out&&port&&pid&&exe&&probes&&rawSource,'Explicit marker/out/port/pid/exe/probes/raw arguments required');
  assert(fs.existsSync(rawSource),'RawFile path must exist (launcher parity; unused by this bounded group)');
  browser=await chromium.connectOverCDP('http://127.0.0.1:'+port,{timeout:30000});
  page=browser.contexts()[0].pages().find(p=>p.url().includes('tauri.localhost'));
  assert(page,'tauri.localhost page missing');
  page.setDefaultTimeout(30000);
  await expect(page.getByTestId('engine-ready')).toContainText(marker,{timeout:30000});
  const identity=await page.evaluate(()=>({testApi:typeof window.__compositor,native:'__TAURI_INTERNALS__' in window}));
  assert.equal(identity.testApi,'undefined','production must not expose __compositor test API');
  assert.equal(identity.native,true);
  page.on('pageerror',e=>errors.push(e.message));

  // --- 1) Blank canvas Vignette: preview / cancel / apply / undo + save/reopen ---
  await newBlankCanvas();
  const blankManifest=await save('blank-before');
  assert.equal(blankManifest.layers.length,1);assert.equal(blankManifest.layers[0].imageFile,undefined,'The tested layer must have no pixels');
  const blankBefore=await picture();
  await openFilter('filter-vignette');
  await field('Amount',70);
  await expect.poll(picture,{timeout:30000}).not.toEqual(blankBefore);
  await page.getByTestId('adjust-cancel').click();
  await expect(page.getByTestId('adjust-panel')).toHaveCount(0);
  assert.equal(await picture(),blankBefore);
  await openFilter('filter-vignette');
  await field('Amount',70);
  await field('Midpoint',40);
  await page.getByTestId('adjust-ok').click();
  await expect(page.getByTestId('adjust-panel')).toHaveCount(0,{timeout:30000});
  await expect(page.getByTestId('working')).toHaveCount(0,{timeout:30000});
  const blankApplied=await picture();
  assert.notEqual(blankApplied,blankBefore);
  await page.keyboard.press('Control+z');
  assert.equal(await picture(),blankBefore);
  await page.keyboard.press('Control+Shift+z');
  assert.equal(await picture(),blankApplied);
  let m=await save('blank-vignette');
  assert.equal(m.layers.length,1);assert(m.layers[0].imageFile,'Applying Vignette must create real saved pixels');
  await close();
  await menu('File','open');
  nativeDialog('select',path.join(out,'blank-vignette.comp'));
  await expect(page.getByTestId('layer-row')).toHaveCount(1,{timeout:30000});
  assert.equal(await picture(),blankApplied);
  await close();
  rows.push({case:'blank Vignette preview/cancel/apply/undo and save/reopen',version:m.version});

  // --- 2) Four new adjustment types: UI edit / reopen / manifest nested settings+version ---
  await importProbe('rgb.png');
  const adjSpecs=[
    {menu:'layer-adjustment-vignette',editKind:'Vignette',field:['Amount',55],nested:(a)=>a?.filterLayerSettings?.vignette},
    {menu:'layer-adjustment-tonal-contrast',editKind:'Tonal Contrast',field:['Shadows',25],nested:(a)=>a?.filterLayerSettings?.tonal},
    {menu:'layer-adjustment-sharpen',editKind:'Sharpen',field:['Amount',150],nested:(a)=>a?.filterLayerSettings?.sharpen},
    {menu:'layer-adjustment-lens-effects',editKind:'Lens Effects',field:null,nested:(a)=>a?.filterLayerSettings?.lens},
  ];
  for(const spec of adjSpecs){
    await menu('Layer',spec.menu);
    await expect(page.getByTestId('adjust-panel')).toBeVisible({timeout:30000});
    if(spec.editKind==='Lens Effects'){
      await page.getByTestId('adjust-panel').getByLabel('Effect',{exact:true}).selectOption('Prism');
      await expect(page.getByTestId('adjust-panel').getByLabel('Amount',{exact:true})).toHaveValue('80');
      await expect(page.getByTestId('adjust-panel').getByLabel('Copies',{exact:true})).toHaveValue('5');
      await expect(page.getByTestId('adjust-panel').getByLabel('Offset',{exact:true})).toHaveValue('6');
      await page.getByTestId('adjust-panel').getByLabel('Direction',{exact:true}).selectOption('Both Ways');
      await field('Blur',12);
      await field('Fringe',40);
    }else if(spec.field){
      await field(spec.field[0],spec.field[1]);
    }
    await page.getByTestId('adjust-ok').click();
    await expect(page.getByTestId('adjust-panel')).toHaveCount(0,{timeout:30000});
    await expect(page.getByTestId('working')).toHaveCount(0,{timeout:30000});
    const stageManifest=await save('adjustment-'+spec.menu);
    assert.equal(stageManifest.version,spec.editKind==='Lens Effects'?20:spec.editKind==='Sharpen'?19:18);
  }
  m=await save('four-adjustments');
  assert.equal(m.version,20);
  const nested=m.layers.map(l=>({name:l.name,kind:l.adjustment?.kind,settings:l.adjustment?adjSpecs.find(s=>s.editKind===l.adjustment.kind)?.nested(l.adjustment):null})).filter(x=>x.kind);
  assert.equal(nested.length,4,'expected four new adjustment layers in manifest');
  for(const n of nested)assert(n.settings,'missing nested filterLayerSettings for '+n.kind);
  assert.equal(nested.find(n=>n.kind==='Vignette').settings.amount,55);
  assert.equal(nested.find(n=>n.kind==='Tonal Contrast').settings.shadows,25);
  assert.equal(nested.find(n=>n.kind==='Sharpen').settings.amount,150);
  const lens=nested.find(n=>n.kind==='Lens Effects');
  assert.equal(lens.settings.effect,'Prism');
  assert.equal(lens.settings.prismAmount,80);
  assert.equal(lens.settings.prismCopies,5);
  assert.equal(lens.settings.prismOffset,6);
  assert.equal(lens.settings.prismBothWays,true);
  assert.equal(lens.settings.prismBlur,12);
  assert.equal(lens.settings.prismFringe,40);
  await close();
  await menu('File','open');
  nativeDialog('select',path.join(out,'four-adjustments.comp'));
  await expect(page.getByTestId('layer-row')).toHaveCount(5,{timeout:30000}); // rgb + 4 adjustments
  for(const spec of adjSpecs){
    await page.getByRole('button',{name:spec.editKind+' adjustment',exact:true}).click();
    await menu('Layer','layer-edit-adjustment');
    await expect(page.getByTestId('adjust-panel')).toBeVisible();
    if(spec.field)await expect(page.getByTestId('adjust-panel').getByLabel(spec.field[0],{exact:true})).toHaveValue(String(spec.field[1]));
    else{
      const panel=page.getByTestId('adjust-panel');
      await expect(panel.getByLabel('Effect',{exact:true})).toHaveValue('Prism');
      await expect(panel.getByLabel('Direction',{exact:true})).toHaveValue('Both Ways');
      await expect(panel.getByRole('spinbutton',{name:'Blur',exact:true})).toHaveValue('12');
      await expect(panel.getByRole('spinbutton',{name:'Fringe',exact:true})).toHaveValue('40');
    }
    await page.getByTestId('adjust-cancel').click();
  }
  await close();
  rows.push({case:'four new adjustment types UI edit/reopen/manifest nested settings+version',version:m.version,kinds:nested.map(n=>n.kind)});

  // --- 3) Prism defaults + Direction Both Ways + Blur/Fringe on destructive filter path ---
  await importProbe('lens.png');
  const lensBefore=await picture();
  await openFilter('filter-lens-effects');
  await page.getByTestId('adjust-panel').getByLabel('Effect',{exact:true}).selectOption('Prism');
  await expect(page.getByTestId('adjust-panel').getByLabel('Amount',{exact:true})).toHaveValue('80');
  await expect(page.getByTestId('adjust-panel').getByLabel('Copies',{exact:true})).toHaveValue('5');
  await expect(page.getByTestId('adjust-panel').getByLabel('Offset',{exact:true})).toHaveValue('6');
  await page.getByTestId('adjust-panel').getByLabel('Direction',{exact:true}).selectOption('Both Ways');
  await field('Blur',8);
  await field('Fringe',35);
  await expect.poll(picture,{timeout:30000}).not.toEqual(lensBefore);
  await page.getByTestId('adjust-ok').click();
  await expect(page.getByTestId('adjust-panel')).toHaveCount(0);
  await expect.poll(picture,{timeout:30000}).not.toEqual(lensBefore);
  await page.keyboard.press('Control+z');
  assert.equal(await picture(),lensBefore);
  await close();
  rows.push({case:'Prism defaults 80/5/6, Direction select Both Ways, Blur/Fringe apply/undo'});

  // Destructive Sharpen shares the adjustment title but must edit the pixel layer.
  await importProbe('lens.png');
  const sharpenBefore=await picture();
  await openFilter('filter-sharpen');await field('Amount',150);await field('Radius',3);
  await expect.poll(picture,{timeout:30000}).not.toEqual(sharpenBefore);
  await page.getByTestId('adjust-cancel').click();assert.equal(await picture(),sharpenBefore);
  await openFilter('filter-sharpen');await field('Amount',150);await field('Radius',3);
  await page.getByTestId('adjust-ok').click();await expect(page.getByTestId('working')).toHaveCount(0);
  await expect.poll(picture,{timeout:30000}).not.toEqual(sharpenBefore);
  await page.keyboard.press('Control+z');assert.equal(await picture(),sharpenBefore);
  await close();rows.push({case:'Sharpen pixel filter preview/cancel/apply/undo'});

  // --- 4) Custom crop valid/invalid/cancel/apply ---
  await importProbe('rgb.png');
  await page.getByTestId('tool-crop').click();
  await expect(page.getByTestId('crop-options')).toBeVisible();
  await expect(page.getByTestId('crop-apply')).toBeEnabled();
  await page.getByLabel('Ratio',{exact:true}).selectOption('Custom');
  await expect(page.getByTestId('crop-custom-ratio')).toBeVisible();
  const customInput=page.getByLabel('Custom ratio width:height',{exact:true});
  await customInput.fill('not-a-ratio');
  await expect(page.getByTestId('crop-custom-apply')).toBeDisabled();
  await page.getByTestId('crop-custom-cancel').click();
  await expect(page.getByTestId('crop-custom-ratio')).toHaveCount(0);
  await page.getByLabel('Ratio',{exact:true}).selectOption('Custom');
  await customInput.fill('4:5');
  await expect(page.getByTestId('crop-custom-apply')).toBeEnabled();
  await page.getByTestId('crop-custom-apply').click();
  await expect(page.getByTestId('crop-custom-ratio')).toHaveCount(0);
  await page.getByTestId('crop-apply').click();
  await expect(page.getByTestId('crop-size')).toHaveText('no selection');
  const cropSaved=await save('crop-four-five');
  assert(Math.abs(cropSaved.width/cropSaved.height-.8)<.05,'Saved canvas must reflect 4:5 crop');
  await page.getByTestId('tool-move').click();
  await page.getByTestId('tool-crop').click();
  await page.getByTestId('crop-cancel').click();
  await expect(page.getByTestId('crop-apply')).toBeDisabled();
  await close();
  rows.push({case:'custom crop invalid disabled / cancel / valid apply'});

  // --- 5) Align/Distribute via real layer selection + move-options buttons; positions in manifest; undo ---
  await importProbe('rgb.png');
  await page.keyboard.press('Control+j');
  await expect(page.getByTestId('layer-row')).toHaveCount(2,{timeout:30000});
  await page.keyboard.press('Control+j');
  await expect(page.getByTestId('layer-row')).toHaveCount(3,{timeout:30000});
  await page.getByTestId('tool-move').click();
  await expect(page.getByTestId('move-options')).toBeVisible();
  // Nudge layers to distinct origins using Move tool + arrow keys (no SetTransform API).
  await selectLayerRows([0]);
  for(let i=0;i<12;i++)await page.keyboard.press('ArrowRight');
  await selectLayerRows([1]);
  for(let i=0;i<8;i++)await page.keyboard.press('ArrowDown');
  await selectLayerRows([2]);
  for(let i=0;i<4;i++){await page.keyboard.press('ArrowRight');await page.keyboard.press('ArrowDown');}
  const beforeAlign=await save('align-before');
  const beforeOrigins=beforeAlign.layers.filter(l=>l.hasPixels||l.imageFile).map(l=>({name:l.name,origin:l.transform.origin}));
  assert.equal(beforeOrigins.length,3);assert.equal(new Set(beforeOrigins.map(o=>o.origin[0])).size,3,'Nudges must produce three distinct x origins');
  await selectLayerRows([0,1,2],{});
  await expect(page.getByTestId('align-Left Edges')).toBeEnabled();
  await page.getByTestId('align-Left Edges').click();
  await expect(page.getByTestId('working')).toHaveCount(0,{timeout:30000});
  const afterAlign=await save('align-left');
  const afterOrigins=afterAlign.layers.filter(l=>l.hasPixels||l.imageFile).map(l=>({name:l.name,origin:l.transform.origin}));
  const xs=afterOrigins.map(o=>o.origin[0]);
  assert(xs.every(x=>x===xs[0]),'Align left edges should equalize origin[0]: '+JSON.stringify(afterOrigins));
  await page.keyboard.press('Control+z');
  await expect(page.getByTestId('working')).toHaveCount(0,{timeout:30000});
  const undone=await save('align-undone');
  const undoneOrigins=undone.layers.filter(l=>l.imageFile).map(l=>({name:l.name,origin:l.transform.origin}));
  assert.deepEqual(undoneOrigins,beforeOrigins,'One undo restores all aligned transforms');
  // Distribute needs >2 selected (MoveOptions.canDistributeSelection).
  await selectLayerRows([0,1,2],{});
  await expect(page.getByTestId('distribute-Horizontal Centers')).toBeEnabled();
  await page.getByTestId('distribute-Horizontal Centers').click();
  await expect(page.getByTestId('working')).toHaveCount(0,{timeout:30000});
  const distributed=await save('distribute-h');
  const dx=distributed.layers.filter(l=>l.imageFile).map(l=>l.transform.origin[0]).sort((a,b)=>a-b);
  const bx=beforeOrigins.map(l=>l.origin[0]).sort((a,b)=>a-b);
  assert.equal(dx.length,3);assert.equal(dx[0],bx[0]);assert.equal(dx[2],bx[2]);assert(Math.abs((dx[1]-dx[0])-(dx[2]-dx[1]))<1e-9,'Equal horizontal center spacing for equal-sized duplicates');
  await page.keyboard.press('Control+z');
  const distributeUndo=await save('distribute-undone');assert.deepEqual(distributeUndo.layers.filter(l=>l.imageFile).map(l=>({name:l.name,origin:l.transform.origin})),beforeOrigins,'One undo restores all distributed transforms');
  await capture('align-distribute');
  rows.push({
    case:'Align/Distribute via move-options buttons; positions verified in manifests; undo',
    beforeOrigins,afterOrigins,
    distributeOrigins:distributed.layers.filter(l=>l.hasPixels||l.imageFile).map(l=>({name:l.name,origin:l.transform.origin})),
    undoneSample:undone.layers.filter(l=>l.hasPixels||l.imageFile).map(l=>({name:l.name,origin:l.transform.origin})),
  });
  await close();

  assert.deepEqual(errors,[]);
  json('result.json',{
    status:'PASS',
    marker,
    identity,
    rows,
    nativeDialogCount:dialogIndex,
    pageErrors:errors,
    scope:'Mac 1.5.11 bounded native group: production WebView2 controls and real native file dialogs. No mocked bridge/test API. Not a 1.5.11 product acceptance claim.',
    acceptanceClaim:false,
  });
  await browser.close();
  console.log('PASS: Mac 1.5.11 bounded native group.');
})().catch(async e=>{
  console.error(e.stack||e);
  try{if(page)await capture('failure');}catch{}
  try{json('failure.json',{error:String(e.stack||e),rows,pageErrors:errors,nativeDialogCount:dialogIndex,exitPending:true});}catch{}
  try{await browser?.close();}catch{}
  process.exitCode=1;
});
