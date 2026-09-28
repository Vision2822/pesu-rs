const $=s=>document.querySelector(s),$$=s=>[...document.querySelectorAll(s)];
const els={
  app:$("#app-container"),loginCard:$("#login-card"),dash:$("#auth-dashboard"),
  form:$("#login-form"),srn:$("#srn-input"),pass:$("#password-input"),
  remember:$("#remember-me"),forget:$("#forget-btn"),loginBtn:$("#login-btn"),
  btnText:$("#login-btn .btn-text"),spin:$("#login-btn .spinner"),err:$("#error-banner"),
  user:$("#user-display"),signout:$("#signout-btn"),
  viewCard:$("#view-card"),viewContent:$("#view-content"),nav:$$(".nav-item"),
  welcome:$("#home-welcome"),sem:$("#profile-sem"),course:$("#profile-course"),
  pLoad:$("#profile-loading-box"),pErr:$("#profile-error-banner"),
  sErr:$("#seating-error-banner"),sLoad:$("#seating-loading-box"),sContent:$("#seating-content"),
  sToday:$("#seating-today"),tCourse:$("#today-course"),tDate:$("#today-date"),tBlock:$("#today-block"),tTerm:$("#today-terminal"),
  sEmpty:$("#seating-empty"),sList:$("#seating-list"),sSummary:$("#seating-summary"),
  semErr:$("#sem-error-banner"),semLoad:$("#sem-loading-box"),semSel:$("#semester-select"),loadSem:$("#load-sem-btn"),
  courseSec:$("#course-section"),loadCoursesBtn:$("#load-courses-btn"),
  cErr:$("#course-error-banner"),cLoad:$("#course-loading-box"),cList:$("#course-list"),cEmpty:$("#no-courses-msg"),
  ttErr:$("#tt-error"),ttLoad:$("#tt-loading"),ttContent:$("#tt-content"),
  ttMetaBar:$("#tt-meta-bar"),ttDaySwitch:$("#tt-day-switch"),ttDayList:$("#tt-day-list"),
  attErr:$("#att-error"),attLoad:$("#att-loading"),attContent:$("#att-content"),
  attList:$("#att-list"),attEmpty:$("#att-empty"),attSummary:$("#att-summary"),attNeedsSem:$("#att-needs-sem"),
};
const invoke=(c,a)=>window.__TAURI__.core.invoke(c,a);
let savedPass=false,curSem=null,profileCache=null,semLoaded=false,seatLoaded=false,ttLoaded=false,attLoaded=false;
const showErr=(el,m)=>{if(!el)return;el.textContent=m||"";el.classList.toggle("hidden",!m);};
const setLoading=v=>{els.loginBtn.disabled=els.srn.disabled=els.pass.disabled=els.remember.disabled=v;els.btnText.textContent=v?"Authenticating...":"Sign In";els.spin.classList.toggle("hidden",!v);if(v)showErr(els.err,"");};
const chip=(k,v,cls="att-summary-chip")=>{const d=document.createElement("div");d.className=cls;d.innerHTML=`<span class="att-summary-k">${k}</span><span class="att-summary-v">${v}</span>`;return d;};

function moveNavIndicator(view){
  const btn=$(`.nav-item[data-view="${view}"]`),ind=$("#nav-indicator"),sb=$("#sidebar");
  if(!btn||!ind||!sb)return;if(btn.getBoundingClientRect().height===0||sb.getBoundingClientRect().height===0)return;
  ind.style.transform=`translateY(${btn.offsetTop}px)`;ind.style.height=`${btn.offsetHeight}px`;
}
function moveIndicator(dayIdx){
  const btn=$(`#tt-day-switch .day-btn[data-day="${dayIdx}"]`),ind=$("#tt-indicator"),sw=$("#tt-day-switch");
  if(!btn||!ind||!sw||sw.offsetParent===null||btn.offsetWidth===0)return;
  ind.style.transform=`translateX(${btn.offsetLeft}px)`;ind.style.width=`${btn.offsetWidth}px`;
}
async function switchView(view){
  els.nav.forEach(b=>b.classList.toggle("active",b.dataset.view===view));
  moveNavIndicator(view);
  const cur=$(".view-inner.active"),next=$(`.view-inner[data-view="${view}"]`);
  if(!next||cur===next){
    if(view==="seating"&&!seatLoaded)loadSeating();
    if(view==="courses"&&!semLoaded)loadSemesters();
    if(view==="timetable"&&!ttLoaded)loadTimetable();
    if(view==="attendance"&&!attLoaded)loadAttendance();
    return;
  }
  if(cur){cur.classList.add("fading");await new Promise(r=>setTimeout(r,160));cur.classList.add("hidden");cur.classList.remove("active","fading");}
  next.classList.remove("hidden");void next.offsetHeight;next.classList.add("active");
  requestAnimationFrame(()=>requestAnimationFrame(()=>{if(view==="timetable"&&ttActiveDay)moveIndicator(ttActiveDay);moveNavIndicator(view);}));
  if(view==="seating")loadSeating();
  if(view==="courses"&&!semLoaded)loadSemesters();
  if(view==="timetable")loadTimetable();
  if(view==="attendance")loadAttendance();
}
els.nav.forEach(b=>b.addEventListener("click",()=>switchView(b.dataset.view)));

let raf=false;
new ResizeObserver(()=>{
  if(raf||els.dash.classList.contains("hidden"))return;raf=true;
  requestAnimationFrame(()=>{
    raf=false;const prev=els.viewCard.style.height;els.viewCard.style.height="auto";
    const nat=els.viewCard.offsetHeight;els.viewCard.style.height=prev;
    const cur=els.viewCard.offsetHeight;if(Math.abs(nat-cur)<16){els.viewCard.style.height=nat+"px";return;}
    els.viewCard.style.height=cur+"px";void els.viewCard.offsetHeight;els.viewCard.style.height=nat+"px";
  });
}).observe(els.viewContent);

async function initCreds(){
  try{const s=await invoke("get_saved_credentials");if(s?.srn){els.srn.value=s.srn;els.remember.checked=true;els.forget.classList.remove("hidden");if(s.has_password){savedPass=true;els.pass.placeholder="•••••••• (Saved)";els.pass.required=false;}}}catch{}
}
async function loadProfile(){
  if(profileCache)return renderProfile(profileCache);
  showErr(els.pErr,"");els.pLoad.classList.remove("hidden");
  try{profileCache=await invoke("get_user_profile");renderProfile(profileCache);}catch(e){showErr(els.pErr,String(e));}finally{els.pLoad.classList.add("hidden");}
}
const renderProfile=p=>{els.welcome.textContent=`Welcome, ${p.name}`;els.user.textContent=p.srn;els.sem.textContent=p.semester;els.course.textContent=p.course;};

async function loadSeating(){
  if(seatLoaded)return;showErr(els.sErr,"");els.sLoad.classList.remove("hidden");els.sContent.classList.add("hidden");
  try{
    const d=await invoke("get_seating");
    if(d.today?.course||d.today?.date||d.today?.block||d.today?.terminal){
      els.tCourse.textContent=d.today.course||"—";els.tDate.textContent=d.today.date||"—";els.tBlock.textContent=d.today.block||"—";els.tTerm.textContent=d.today.terminal||"—";els.sToday.classList.remove("hidden");
    }else els.sToday.classList.add("hidden");
    els.sList.innerHTML="";els.sSummary.innerHTML="";
    const entries=d.entries||[];
    if(!entries.length){els.sList.classList.add("hidden");els.sSummary.classList.add("hidden");els.sEmpty.classList.remove("hidden");}
    else{
      els.sEmpty.classList.add("hidden");
      const total=entries.length,assess=[...new Set(entries.map(e=>e.assessment).filter(Boolean))].length;
      [["Total",total],["Assessments",assess||"—"],["Today",d.today?.course?"Yes":"—"]].forEach(([k,v])=>els.sSummary.appendChild(chip(k,v)));
      els.sSummary.classList.remove("hidden");
      entries.forEach((e,i)=>{
        const card=document.createElement("div");card.className="seating-card";card.style.animationDelay=`${i*38}ms`;
        card.innerHTML=`<div class="seating-card-top"><div class="seating-card-head"><span class="seating-assessment">${e.assessment||"Assessment"}</span><span class="seating-course">${e.course_code||"—"}</span></div><div class="seating-card-datebox"><div class="seating-date">${e.date||"—"}</div>${e.time?`<div class="seating-time">${e.time}</div>`:""}</div></div><div class="seating-card-bottom"><div class="seat-big block"><span class="seat-big-k">Block</span><span class="seat-big-v">${e.block||"—"}</span></div><div class="seat-big terminal"><span class="seat-big-k">Terminal</span><span class="seat-big-v">${e.terminal||"—"}</span></div></div>`;
        els.sList.appendChild(card);
      });
      els.sList.classList.remove("hidden");
    }
    seatLoaded=true;
  }catch(e){showErr(els.sErr,String(e));seatLoaded=false;}finally{els.sLoad.classList.add("hidden");els.sContent.classList.remove("hidden");}
}

async function loadSemesters(){
  showErr(els.semErr,"");els.semLoad.classList.remove("hidden");els.semSel.disabled=true;els.loadSem.classList.add("hidden");
  try{
    const sems=await invoke("get_semesters");els.semSel.innerHTML="";
    if(!sems?.length){showErr(els.semErr,"No semesters found.");els.loadSem.classList.remove("hidden");return;}
    sems.forEach(s=>{const o=document.createElement("option");o.value=s.id;o.textContent=s.name;els.semSel.appendChild(o);});
    els.semSel.disabled=false;semLoaded=true;
    let pick=sems[0].id;const home=profileCache?.semester||els.sem?.textContent||"";const m=home.match(/(\d+)/);
    if(m){const num=m[1],found=sems.find(s=>s.id===num||s.name.includes(num)||s.name.toLowerCase().includes(`sem-${num}`)||s.name.toLowerCase().includes(`sem ${num}`));if(found)pick=found.id;}
    els.semSel.value=pick;curSem=pick;els.courseSec.classList.add("expanded");attLoaded=false;
    loadCourses(pick);setTimeout(()=>loadAttendance().catch(()=>{}),800);
  }catch(e){showErr(els.semErr,String(e));els.loadSem.classList.remove("hidden");}finally{els.semLoad.classList.add("hidden");}
}
async function loadCourses(id){
  showErr(els.cErr,"");els.cLoad.classList.remove("hidden");els.cList.innerHTML="";els.cEmpty.classList.add("hidden");els.loadCoursesBtn.disabled=true;
  try{
    const courses=await invoke("get_courses",{semesterId:id});
    if(courses?.length)courses.forEach(c=>{const d=document.createElement("div");d.className="course-item";d.innerHTML=`<div class="course-code">${c.code}</div><div class="course-title">${c.title}</div>`;d.onclick=()=>d.classList.toggle("selected");els.cList.appendChild(d);});
    else els.cEmpty.classList.remove("hidden");
  }catch(e){showErr(els.cErr,String(e));}finally{els.cLoad.classList.add("hidden");els.loadCoursesBtn.disabled=false;}
}
async function loadTimetable(){
  if(ttLoaded)return;showErr(els.ttErr,"");els.ttLoad.classList.remove("hidden");els.ttContent.classList.add("hidden");
  try{const data=await invoke("get_timetable");renderTimetable(data);ttLoaded=true;}catch(e){showErr(els.ttErr,String(e));ttLoaded=false;}finally{els.ttLoad.classList.add("hidden");}
}
async function loadAttendance(){
  let semId=curSem||els.semSel?.value||(semLoaded&&els.semSel?.options[0]?.value);
  if(!semId&&!semLoaded){await loadSemesters();semId=curSem||els.semSel?.value;}
  if(!semId){if(els.attNeedsSem)els.attNeedsSem.classList.remove("hidden");return;}
  if(els.attNeedsSem)els.attNeedsSem.classList.add("hidden");
  if(attLoaded)return;
  showErr(els.attErr,"");els.attLoad.classList.remove("hidden");els.attContent.classList.add("hidden");
  try{const data=await invoke("get_attendance",{semesterId:semId});renderAttendance(data);attLoaded=true;}catch(e){showErr(els.attErr,String(e));attLoaded=false;}finally{els.attLoad.classList.add("hidden");}
}
function renderAttendance(data){
  const recs=data?.records||data||[];els.attList.innerHTML="";els.attSummary.innerHTML="";
  if(!recs.length){els.attEmpty.classList.remove("hidden");els.attContent.classList.remove("hidden");els.attSummary.classList.add("hidden");return;}
  els.attEmpty.classList.add("hidden");
  let a=0,t=0,below=0,pt=0,pc=0;
  recs.forEach(r=>{if(r.attended!=null&&r.total!=null){a+=r.attended;t+=r.total;}if(r.percentage!=null){pt+=r.percentage;pc++;if(r.percentage<75)below++;}});
  const overall=t>0?a/t*100:pc?pt/pc:0;
  [["Overall",overall?`${overall.toFixed(1)}%`:"—"],["Courses",recs.length],["Below 75%",below],["Total",t?`${a}/${t}`:"—"]].forEach(([k,v])=>els.attSummary.appendChild(chip(k,v)));
  els.attSummary.classList.remove("hidden");
  recs.forEach((r,i)=>{
    const pct=r.percentage;let status="na",bar="na",pcCls="na",label="NA";
    if(pct!=null){if(pct>=85){status="good";bar="good";pcCls="good";label="Safe";}else if(pct>=75){status="warn";bar="warn";pcCls="warn";label="Borderline";}else{status="bad";bar="bad";pcCls="bad";label="Shortage";}}
    let bunk="";if(r.attended!=null&&r.total!=null&&r.total>0){
      if(pct!=null&&pct>=75){const can=Math.floor(r.attended/0.75-r.total);bunk=can>0?`Can miss ${can}`:can===0?"At edge":"";}
      else if(pct!=null){const need=Math.ceil((0.75*r.total-r.attended)/0.25);if(need>0)bunk=`Need ${need} more`;}
    }
    const el=document.createElement("div");el.className="att-item";el.style.animationDelay=`${i*38}ms`;
    const raw=r.raw_classes||(r.attended!=null&&r.total!=null?`${r.attended}/${r.total}`:"—");
    el.innerHTML=`<div class="att-item-left"><div class="att-item-head"><span class="att-code">${r.code||"—"}</span><span class="att-status ${status}">${label}</span></div><div class="att-name">${r.name||r.code||"Untitled"}</div><div class="att-meta"><span>${raw}</span>${bunk?`<span>• ${bunk}</span>`:""}</div><div class="att-bar-wrap"><div class="att-bar ${bar}" style="width:${pct!=null?Math.min(100,Math.max(0,pct)):0}%"></div></div></div><div class="att-item-right"><div class="att-pct ${pcCls}">${pct!=null?Math.round(pct)+"<span style='font-size:.7em;opacity:.7'>%</span>":"—"}</div><div class="att-pct-sub">${raw}</div></div>`;
    els.attList.appendChild(el);
  });
  els.attContent.classList.remove("hidden");
}

let ttDataCache=null,ttActiveDay=1;
function renderTimetable(data){
  ttDataCache=data;els.ttContent.classList.remove("hidden");
  const m=data.meta||{};els.ttMetaBar.innerHTML="";
  const meta=[["Batch",m.batch],["Class",m.class_name],["Dept",m.department],["Section",m.section],["Room",m.room]].filter(([,v])=>v&&v.trim());
  if(meta.length){meta.forEach(([k,v])=>{const chipEl=document.createElement("div");chipEl.className="tt-meta-chip";chipEl.innerHTML=`<span class="tt-meta-k">${k}</span><span class="tt-meta-v">${v}</span>`;els.ttMetaBar.appendChild(chipEl);});els.ttMetaBar.classList.remove("hidden");}else els.ttMetaBar.classList.add("hidden");
  const days=data.days||[];els.ttDaySwitch.innerHTML="";
  const ind=document.createElement("div");ind.id="tt-indicator";ind.className="day-indicator";els.ttDaySwitch.appendChild(ind);
  const shorts=["Mon","Tue","Wed","Thu","Fri","Sat"];const js=new Date().getDay();let today=(js>=1&&js<=6)?js:1;
  days.forEach((d,i)=>{const idx=d.index||i+1,short=shorts[i]||d.day.slice(0,3);const b=document.createElement("button");b.type="button";b.className="day-btn";b.dataset.day=idx;b.textContent=short;if(idx===today)b.classList.add("today");b.addEventListener("click",()=>setActiveDay(idx,true));els.ttDaySwitch.appendChild(b);});
  setActiveDay(today,false);
  requestAnimationFrame(()=>requestAnimationFrame(()=>{moveIndicator(today);setTimeout(()=>moveIndicator(today),50);}));
}
function setActiveDay(dayIdx,animate=true){
  const prev=ttActiveDay;ttActiveDay=dayIdx;
  $$("#tt-day-switch .day-btn").forEach(b=>b.classList.toggle("active",Number(b.dataset.day)===dayIdx));
  moveIndicator(dayIdx);
  if(!ttDataCache)return;
  const dayObj=(ttDataCache.days||[]).find(d=>d.index===dayIdx),slots=ttDataCache.slots||[];
  if(animate&&prev!==dayIdx&&els.ttDayList.children.length){els.ttDayList.classList.add("switching");setTimeout(()=>{renderDayList(dayObj,slots);els.ttDayList.classList.remove("switching");},160);}else renderDayList(dayObj,slots);
}
function renderDayList(dayObj,slots){
  els.ttDayList.innerHTML="";if(!dayObj){els.ttDayList.innerHTML=`<div class="info-text">No data for this day</div>`;return;}
  const filtered=dayObj.slots.filter(sd=>{const si=slots.find(s=>s.ordered_by===sd.ordered_by);if(!si)return false;if(si.is_break&&si.start==="12:00 AM"&&si.end==="12:00 AM")return false;return true;});
  let last=-1;for(let i=filtered.length-1;i>=0;i--){if(!filtered[i].is_break&&filtered[i].entries?.length){last=i;break;}}
  if(last===-1){els.ttDayList.innerHTML=`<div class="info-text">No classes today — enjoy your day!</div>`;return;}
  filtered.slice(0,last+1).forEach((sd,i)=>{
    const si=slots.find(s=>s.ordered_by===sd.ordered_by),start=si?.start||"",end=si?.end||"";
    let el;if(sd.is_break){el=document.createElement("div");el.className="tt-break-card";el.style.animationDelay=`${i*28}ms`;el.innerHTML=`<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M18 8h1a4 4 0 0 1 0 8h-1"/><path d="M2 8h16v9a4 4 0 0 1-4 4H6a4 4 0 0 1-4-4V8z"/><line x1="6" y1="1" x2="6" y2="4"/><line x1="10" y1="1" x2="10" y2="4"/><line x1="14" y1="1" x2="14" y2="4"/></svg><span>Break</span><span style="opacity:.6">${start} - ${end}</span>`;}
    else{const entries=sd.entries||[];el=document.createElement("div");el.className="tt-item";el.style.animationDelay=`${i*38}ms`;
      const timeCol=`<div class="tt-time"><span class="tt-time-start">${start}</span><span class="tt-time-end">${end}</span></div>`;
      let html=entries.length?entries.map(en=>`<div class="tt-entry"><div class="tt-code">${en.code||""}</div><div class="tt-title">${en.title||en.code||"Untitled"}</div>${en.faculty?`<div class="tt-faculty">${en.faculty}</div>`:""}</div>`).join(""):`<span class="tt-empty">No class</span>`;
      el.innerHTML=timeCol+`<div>${html}</div>`;
    }
    els.ttDayList.appendChild(el);
  });
}
window.addEventListener("resize",()=>{if(ttActiveDay)moveIndicator(ttActiveDay);const v=$(".nav-item.active")?.dataset.view;if(v)moveNavIndicator(v);});

// settings
const SETTINGS_KEY="pesu-rs-settings",defaultSettings={accent:"#2a7fff",density:"compact",font:"medium"};
const loadSettings=()=>{try{return{...defaultSettings,...JSON.parse(localStorage.getItem(SETTINGS_KEY)||"{}")};}catch{return{...defaultSettings};}};
const saveSettings=s=>{localStorage.setItem(SETTINGS_KEY,JSON.stringify(s));applySettings(s);};
function applySettings(s){
  document.documentElement.style.setProperty("--accent",s.accent);
  document.documentElement.style.setProperty("--accent-soft",s.accent.startsWith("#")?hexToRgba(s.accent,0.35):"rgba(42,127,255,.35)");
  document.documentElement.dataset.density=s.density;document.documentElement.dataset.font=s.font;
  const sc=s.font==="small"?0.92:s.font==="large"?1.08:1;
  document.documentElement.style.setProperty("--font-scale",sc);document.documentElement.style.fontSize=`${15*sc}px`;
  document.documentElement.style.setProperty("--r",s.density==="cozy"?"12px":"10px");
  document.documentElement.style.setProperty("--r-lg",s.density==="cozy"?"18px":"14px");
  $$("#accent-dots .color-dot").forEach(b=>{if(b.dataset.accent)b.classList.toggle("active",b.dataset.accent.toLowerCase()===s.accent.toLowerCase());});
  const cp=$("#custom-accent");if(cp)cp.value=s.accent.length===7?s.accent:"#2a7fff";
  $$("#density-switch .seg-btn").forEach(b=>b.classList.toggle("active",b.dataset.density===s.density));
  $$("#font-switch .seg-btn").forEach(b=>b.classList.toggle("active",b.dataset.font===s.font));
  requestAnimationFrame(()=>requestAnimationFrame(()=>{const v=$(".nav-item.active")?.dataset.view;if(v)moveNavIndicator(v);if(ttActiveDay)moveIndicator(ttActiveDay);}));
}
const hexToRgba=(hex,a)=>{const h=hex.replace("#","").trim();if(h.length===3){const r=parseInt(h[0]+h[0],16),g=parseInt(h[1]+h[1],16),b=parseInt(h[2]+h[2],16);return`rgba(${r},${g},${b},${a})`;}if(h.length===6){const r=parseInt(h.slice(0,2),16),g=parseInt(h.slice(2,4),16),b=parseInt(h.slice(4,6),16);return`rgba(${r},${g},${b},${a})`;}return`rgba(42,127,255,${a})`;};
function initSettings(){
  const s=loadSettings();applySettings(s);
  $("#accent-dots")?.addEventListener("click",e=>{const b=e.target.closest("[data-accent]");if(!b)return;const ns={...loadSettings(),accent:b.dataset.accent};saveSettings(ns);const cp=$("#custom-accent");if(cp)cp.value=b.dataset.accent;});
  $("#custom-accent")?.addEventListener("input",e=>saveSettings({...loadSettings(),accent:e.target.value}));
  $("#density-switch")?.addEventListener("click",e=>{const b=e.target.closest("[data-density]");if(!b)return;saveSettings({...loadSettings(),density:b.dataset.density});});
  $("#font-switch")?.addEventListener("click",e=>{const b=e.target.closest("[data-font]");if(!b)return;saveSettings({...loadSettings(),font:b.dataset.font});});
  $("#toggle-fullscreen")?.addEventListener("click",async()=>{try{if(window.__TAURI__){const w=window.__TAURI__.window.getCurrentWindow(),fs=await w.isFullscreen();await w.setFullscreen(!fs);}else{if(!document.fullscreenElement)document.documentElement.requestFullscreen();else document.exitFullscreen();}}catch(err){console.warn(err);}});
  $("#clear-creds")?.addEventListener("click",async()=>{try{await invoke("forget_saved_credentials");alert("Saved credentials cleared");}catch(e){alert(String(e));}});
}

els.pass.addEventListener("input",()=>{if(savedPass&&els.pass.value){savedPass=false;els.pass.placeholder="Password";els.pass.required=true;}});
els.forget.addEventListener("click",async()=>{try{await invoke("forget_saved_credentials");}catch(e){showErr(els.err,String(e));return;}els.srn.value=els.pass.value="";els.pass.placeholder="Password";els.pass.required=true;els.remember.checked=false;savedPass=false;els.forget.classList.add("hidden");showErr(els.err,"");});
els.form.addEventListener("submit",async e=>{
  e.preventDefault();showErr(els.err,"");const srn=els.srn.value.trim().toUpperCase(),pass=els.pass.value,rem=els.remember.checked;
  if(!srn)return showErr(els.err,"Please enter your SRN.");if(!pass&&!savedPass)return showErr(els.err,"Please enter your password.");
  setLoading(true);
  try{
    const r=await invoke("login",{srn,password:savedPass&&!pass?"":pass,rememberMe:rem});
    if(r?.success!==false){
      if(!rem){savedPass=false;els.forget.classList.add("hidden");}
      els.loginCard.classList.add("hidden");els.dash.classList.remove("hidden");els.viewCard.style.height=els.viewCard.offsetHeight+"px";
      els.signout.classList.remove("hidden");els.app.classList.add("authed");
      requestAnimationFrame(()=>requestAnimationFrame(()=>moveNavIndicator("home")));
      switchView("home");loadProfile();setTimeout(()=>loadTimetable().catch(()=>{}),600);
    }
  }catch(err){showErr(els.err,String(err));}finally{setLoading(false);}
});
els.semSel.addEventListener("change",async()=>{
  const id=els.semSel.value;if(!id)return;try{await invoke("set_selected_semester",{id});curSem=id;els.courseSec.classList.add("expanded");attLoaded=false;loadCourses(id);loadAttendance();}catch(err){showErr(els.semErr,String(err));}
});
els.loadSem.addEventListener("click",loadSemesters);
els.loadCoursesBtn.addEventListener("click",()=>curSem&&loadCourses(curSem));
$("#att-reload")?.addEventListener("click",()=>{attLoaded=false;loadAttendance();});
$("#seating-reload")?.addEventListener("click",()=>{seatLoaded=false;loadSeating();});
els.signout.addEventListener("click",async()=>{
  try{await invoke("logout");}catch{}els.dash.classList.add("hidden");els.viewCard.style.height="";els.signout.classList.add("hidden");
  els.loginCard.classList.remove("hidden");els.app.classList.remove("authed");els.semSel.innerHTML="";els.courseSec.classList.remove("expanded");
  els.cList.innerHTML="";els.cEmpty.classList.add("hidden");curSem=profileCache=null;semLoaded=seatLoaded=ttLoaded=attLoaded=false;
  els.attList.innerHTML="";els.attSummary.innerHTML="";els.attContent.classList.add("hidden");els.attEmpty.classList.add("hidden");els.attNeedsSem.classList.remove("hidden");
  els.welcome.textContent="Welcome";els.user.textContent=els.sem.textContent=els.course.textContent="—";
  els.sToday.classList.add("hidden");els.sEmpty.classList.add("hidden");els.sList.innerHTML="";els.sList.classList.add("hidden");els.sSummary.innerHTML="";els.sSummary.classList.add("hidden");
  els.ttContent.classList.add("hidden");els.ttMetaBar.innerHTML="";els.ttDaySwitch.innerHTML="";els.ttDayList.innerHTML="";
  [els.err,els.pErr,els.semErr,els.cErr,els.sErr,els.ttErr,els.attErr].forEach(el=>showErr(el,""));
  initCreds();
});
window.addEventListener("DOMContentLoaded",()=>{initCreds();initSettings();requestAnimationFrame(()=>moveNavIndicator("home"));});
