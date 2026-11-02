/* GitHub Pages UI demo. Test data stays in this browser; no Rust service is emulated remotely. */
(() => {
  const KEY='everafter.github-demo.v1';
  const clone=v=>JSON.parse(JSON.stringify(v));
  const id=()=>crypto.randomUUID();
  const groups={
    categories:[{id:'family',name:'Family'},{id:'friends',name:'Friends'},{id:'colleagues',name:'Colleagues'},{id:'other',name:'Other'}],
    tiers:[{id:'inner-circle',name:'Tier 1 · Inner circle',rank:1,hotelSupport:true,travelSupport:true,description:'Close family and friends.'},{id:'extended-circle',name:'Tier 2 · Extended circle',rank:2,hotelSupport:true,travelSupport:false,description:'Guests offered accommodation.'},{id:'wider-circle',name:'Tier 3 · Wider circle',rank:3,hotelSupport:false,travelSupport:false,description:'Self-arranged stays and travel.'}]
  };
  function seed(){return {settings:[{id:'wedding',names:'Your names',date:'2027-03-21',venue:'The Garden Pavilion',time:'5:00 PM',dress:'Festive attire',budget:1000000,weddingKind:'Choose later'}],...clone(groups),guests:[{id:'demo-g1',name:'Asha · sample guest',email:'asha@example.com',category:'Family',tier:groups.tiers[0].name,rsvp:'Attending',meal:'Vegetarian',side:'Partner 1',needsHotel:true,needsRide:true,passengers:2},{id:'demo-g2',name:'Dev · sample guest',email:'dev@example.com',category:'Friends',tier:groups.tiers[1].name,rsvp:'Pending',side:'Partner 2'},{id:'demo-g3',name:'Riya · sample guest',email:'riya@example.com',category:'Colleagues',tier:groups.tiers[2].name,rsvp:'Declined',side:'Shared'}],tasks:[{id:'demo-t1',dependencyIds:['demo-t3'],name:'Shortlist your favourite venues',due:'2026-12-01',owner:'Together',done:false},{id:'demo-t2',dependencyIds:['demo-t1'],name:'Choose an invitation style',due:'2027-01-10',owner:'Mira',done:false},{id:'demo-t3',name:'Agree on your wedding priorities',due:'2026-11-01',owner:'Together',done:true}],vendors:[{id:'demo-v1',name:'Sample Garden Florals',category:'Flowers',status:'Considering',quote:45000,paid:0,due:'2027-02-20',next:'Confirm palette and final quote'}],rides:[]}}
  let state;
  try {state=JSON.parse(localStorage.getItem(KEY)||'null')||seed()}catch{state=seed()}
  window.plannerDemo.migrate(state);
  function persist(){try{localStorage.setItem(KEY,JSON.stringify(state))}catch{throw Error('Browser storage is unavailable or full. Export your guests before resetting the demo.')}}
  const labelKey=(s,phone=false)=>phone?String(s||'').replace(/\D/g,'').replace(/^00/,''):String(s||'').trim().toLowerCase();
  function canonical(kind,name){if(!name)return '';const clean=String(name).trim();let group=state[kind].find(g=>g.name.toLowerCase()===clean.toLowerCase());if(!group){group={id:id(),name:clean,description:'Imported label',rank:99,hotelSupport:false,travelSupport:false};state[kind].push(group)}return group.name}
  function addStarters(record,old){if(!record.weddingKind||record.weddingKind==='Choose later'||record.weddingKind===old?.weddingKind)return;const bundle=window.EVERAFTER_STARTERS,templates=bundle?.templates[record.weddingKind];if(!templates)throw Error('Checklist templates could not load. Refresh the page.');for(const template of templates){if(state.tasks.some(t=>t.id===template.id))continue;const task=clone(template);if(record.date){const offset=(Date.parse(task.due+'T12:00:00Z')-Date.parse(bundle.baseDate+'T12:00:00Z'))/86400000;task.due=new Date(Date.parse(record.date+'T12:00:00Z')+offset*86400000).toISOString().slice(0,10)}else task.due='';state.tasks.push(task)}}
  window.everafterDemoApi=async function(url,method='GET',body){
    if(url==='/api/ai/status')return {connected:false,model:'Unavailable in browser demo'};
    if(url==='/api/contacts/suggest')throw Error('AI categorization needs the local Rust app and an API key. This demo uses local label-based suggestions.');
    if(url.startsWith('/api/ai/')||url.startsWith('/api/invitations/generate'))throw Error('AI generation needs the Rust backend. API keys are not accepted in this browser-only demo.');
    if(url==='/api/invitations')return [];
    if(url.startsWith('/api/guest/'))throw Error('Private RSVP links require the Rust server and are unavailable in this demo.');
    if(url==='/api/records'&&method==='GET')return clone(state);
    const before=clone(state);
    try {
      if(url==='/api/accommodation/auto-allocate'){const result=window.plannerDemo.autoAllocate(state);persist();return result;}
      if(url==='/api/transport/assign'){const result=window.plannerDemo.assignTransport(state,body);persist();return result;}
      if(url==='/api/guests/organize'){
        if(!body.ids?.length)throw Error('Select at least one guest.');
        for(const key of ['category','tier'])if(key in body&&body[key]&&!state[key==='category'?'categories':'tiers'].some(g=>g.name===body[key]))throw Error('That group no longer exists.');
        for(const guestId of body.ids){const g=state.guests.find(g=>g.id===guestId);if(!g)throw Error('A selected guest was removed.');for(const key of ['category','tier'])if(key in body)g[key]=body[key];if('category'in body)g.categoryReview='confirmed'}
        persist();return {ok:true};
      }
      if(url==='/api/guests/import'){
        const seen=new Set();for(const g of state.guests)for(const k of ['phone','email']){const key=labelKey(g[k],k==='phone');if(key)seen.add(k+':'+key)}let imported=0,skipped=0;
        for(const c of body.contacts||[]){if(!c.name?.trim())throw Error('Each contact needs a name.');const keys=['phone','email'].map(k=>[k,labelKey(c[k],k==='phone')]).filter(([,v])=>v).map(([k,v])=>k+':'+v);if(keys.some(k=>seen.has(k))){skipped++;continue}const g={...clone(c),id:id(),rsvp:'Pending',contactImport:true};g.category=canonical('categories',g.category);g.tier=canonical('tiers',g.tier);state.guests.push(g);keys.forEach(k=>seen.add(k));imported++}persist();return {imported,skipped};
      }
      const match=url.match(/^\/api\/records\/([^/]+)(?:\/([^/]+))?$/);if(!match)throw Error('This operation is not available in the demo.');const [,kind,recordId]=match;if(!Array.isArray(state[kind]))throw Error('Unknown record type.');
      if(method==='DELETE'){window.plannerDemo.beforeDelete(state,kind,recordId);const oldAllocation=kind==='allocations'?state.allocations.find(a=>a.id===recordId):null;
        if(['categories','tiers'].includes(kind)){const group=state[kind].find(g=>g.id===recordId),field=kind==='tiers'?'tier':'category';if(group&&state.guests.some(g=>g[field]===group.name))throw Error('Reassign guests before deleting this group.');if(kind==='tiers'&&group&&state.bookings.some(b=>b.eligibleTiers?.includes(group.name)))throw Error('Update booking eligibility before deleting this tier.')}
        if(kind==='rides'&&state.guests.some(g=>g.pickup===recordId))throw Error('Reassign guests before deleting this vehicle.');
        state[kind]=state[kind].filter(g=>g.id!==recordId);if(oldAllocation)window.plannerDemo.syncGuest(state,oldAllocation.guestId);persist();return {ok:true};
      }
      if(method==='POST'){
        const record={...clone(body),id:body.id||id()},old=state[kind].find(g=>g.id===record.id);
        window.plannerDemo.validate(state,kind,record);
        if(['categories','tiers'].includes(kind)){
          record.name=String(record.name||'').trim();if(!record.name||record.name.length>120)throw Error('Enter a group name under 120 characters.');
          if(state[kind].some(g=>g.id!==record.id&&g.name.toLowerCase()===record.name.toLowerCase()))throw Error('That group name already exists.');
          if(kind==='tiers'&&(!Number.isInteger(record.rank)||record.rank<1||record.rank>999))throw Error('Tier priority must be a whole number from 1 to 999.');
          if(old){const field=kind==='tiers'?'tier':'category';for(const g of state.guests)if(g[field]===old.name)g[field]=record.name;if(kind==='tiers')for(const b of state.bookings)b.eligibleTiers=(b.eligibleTiers||[]).map(t=>t===old.name?record.name:t)}
        }
        if(kind==='guests'){record.category=canonical('categories',record.category);record.tier=canonical('tiers',record.tier)}
        if(kind==='settings')addStarters(record,old);
        const i=state[kind].findIndex(g=>g.id===record.id);if(i>=0)state[kind][i]=record;else state[kind].push(record);window.plannerDemo.afterSave(state,kind,record,old);persist();return clone(record);
      }
      throw Error('Unsupported demo operation.');
    } catch(e){state=before;throw e}
  };
  window.demoFeedback=function(){let repo=window.EVERAFTER_REPOSITORY;if(!repo&&location.hostname.endsWith('.github.io')){const owner=location.hostname.split('.')[0],name=location.pathname.split('/').filter(Boolean)[0];if(name)repo='https://github.com/'+owner+'/'+name}if(!repo){alert('The GitHub repository link will be added during publication.');return}window.open(repo+'/issues/new/choose','_blank','noopener')};
  window.resetDemo=function(){if(!confirm('Reset the fictional sample wedding in this browser? This clears this demo’s local test changes.'))return;state=seed();persist();location.reload()};
  window.addEventListener('DOMContentLoaded',()=>{
    document.title='Everafter · Interactive test demo';
    const banner=document.createElement('div');banner.id='demo-test-banner';banner.innerHTML='<span>TEST DEMO · Saves only in this browser</span><button onclick="demoFeedback()">Feedback ↗</button><button onclick="resetDemo()" aria-label="Reset sample data">↺</button>';document.body.append(banner);
    const style=document.createElement('style');style.textContent='#demo-test-banner{position:fixed;bottom:10px;left:250px;right:20px;z-index:7;background:#293e34;color:#fffefa;border-radius:10px;padding:9px 12px;display:flex;align-items:center;gap:12px;font:10px system-ui;box-shadow:0 3px 16px #293e3420}#demo-test-banner span{flex:1}#demo-test-banner button{border:0;background:none;color:white;font:11px system-ui;cursor:pointer}@media(max-width:700px){#demo-test-banner{top:0;bottom:auto;left:0;right:0;height:29px;padding:5px 10px;border-radius:0;font-size:8px}aside{top:29px}main{top:86px}}@media print{#demo-test-banner{display:none}}';document.head.append(style);
  });
})();
