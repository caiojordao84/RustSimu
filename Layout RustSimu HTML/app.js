(()=>{
"use strict";
const $=(s,r=document)=>r.querySelector(s), $$=(s,r=document)=>[...r.querySelectorAll(s)];
const app=$("#app"), svg=$("#sheet"), vp=$("#viewport");
const SHEET={w:1040,h:720};

/* ---------- Tema: system → light → dark ---------- */
const mq=matchMedia("(prefers-color-scheme: light)");
const setTheme=m=>{localStorage.setItem("rs-theme",m);document.documentElement.dataset.theme=m==="system"?(mq.matches?"light":"dark"):m;};
mq.addEventListener("change",()=>{if((localStorage.getItem("rs-theme")||"system")==="system")setTheme("system");});

/* ---------- Biblioteca (substituir pelo catálogo real) ---------- */
const LIB=[
 {c:"Alimentações & Linhas (L1, L2, L3, PE)",i:[["L1-L2-L3","Barramento Trifásico",3,"gen"],["N + PE","Neutro e Terra",2,"gen"],["+ / -","Corrente Contínua (CC)",2,"gen"]]},
 {c:"Fusíveis & Disjuntores",i:[["Q1","Disjuntor-Motor Magnetotérmico",6,"brk"],["F1","Fusíveis Seccionadores",6,"gen"],["RT","Relé Térmico de Sobrecarga",6,"gen"]]},
 {c:"Contatores & Bobinas (K1, K2…)",i:[["KM","Contator Tripolar Principal",8,"ctr"],["A1-A2","Bobina de Contator 24V/220V",2,"gen"],["NA/NF","Contatos Auxiliares de Selo",4,"ctr"]]},
 {c:"Motores Elétricos",i:[["M 3~","Motor de Indução Trifásico",6,"gen"],["M 1~","Motor Monofásico",4,"gen"],["M CC","Motor Corrente Contínua",2,"gen"]]},
 {c:"Transformadores & Retificadores",i:[["~ / =","Ponte Retificadora Diodos",4,"gen"],["TR","Trafo Rebaixador/Isolador",4,"gen"]]},
 {c:"Botoeiras & Sensores",i:[["S1","Botão Pulsador NA (Start)",2,"gen"],["S0","Botão Desliga NF (Stop)",2,"gen"],["S_EM","Cogumelo de Emergência",2,"gen"]]}
];
const SYMS={
 brk:'<svg viewBox="0 0 150 104" width="150" height="104" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><g><path d="M30 8v14M30 22l-10 34M30 56v0M30 56v14"/><path d="M75 8v14M75 22l-10 34M75 56v14"/><path d="M120 8v14M120 22l-10 34M120 56v14"/><path stroke-dasharray="4 3" stroke="#FFB300" d="M20 40h110"/><rect x="24" y="70" width="12" height="16"/><rect x="69" y="70" width="12" height="16"/><rect x="114" y="70" width="12" height="16"/><path d="M30 86v10M75 86v10M120 86v10"/></g></svg>',
 ctr:'<svg viewBox="0 0 150 104" width="150" height="104" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><path d="M30 8v26M30 34l-10 32M30 66v30M75 8v26M75 34l-10 32M75 66v30M120 8v26M120 34l-10 32M120 66v30"/><path stroke-dasharray="4 3" d="M20 50h110"/></svg>',
 gen:'<svg viewBox="0 0 150 104" width="150" height="104" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><rect x="40" y="22" width="70" height="60" rx="2"/><path d="M75 8v14M75 82v14"/></svg>'
};
const tree=$("#tree");
let selected=null;
function buildTree(){
 tree.innerHTML="";
 LIB.forEach((g,gi)=>{
  const li=document.createElement("li");li.setAttribute("role","none");
  li.innerHTML=`<button class="t-cat" type="button" role="treeitem" aria-expanded="true" aria-controls="grp-${gi}"><svg class="icon chev" aria-hidden="true"><use href="#i-chev"/></svg><span>${g.c}</span></button><ul id="grp-${gi}" role="group"></ul>`;
  const ul=$("ul",li);
  g.i.forEach((it,ii)=>{
   const l=document.createElement("li");l.setAttribute("role","none");
   l.innerHTML=`<button class="t-item" type="button" role="treeitem" aria-selected="false" data-g="${gi}" data-i="${ii}"><span class="tag">${it[0]}</span><span class="name">${it[1]}</span></button>`;
   ul.appendChild(l);
  });
  tree.appendChild(li);
 });
 select(1,0);
}
function select(gi,ii){
 const it=LIB[gi].i[ii];selected=it;
 $$(".t-item",tree).forEach(b=>b.setAttribute("aria-selected",String(+b.dataset.g===gi&&+b.dataset.i===ii)));
 $("#preview-box").innerHTML=SYMS[it[3]];
 $("#preview-name").textContent=`${it[0]}: ${it[1]}`;
 $("#f-desig").value=it[1];
}
tree.addEventListener("click",e=>{
 const cat=e.target.closest(".t-cat");
 if(cat){const open=cat.getAttribute("aria-expanded")==="true";cat.setAttribute("aria-expanded",String(!open));$("#"+cat.getAttribute("aria-controls")).hidden=open;return;}
 const it=e.target.closest(".t-item");if(it)select(+it.dataset.g,+it.dataset.i);
});
$("#filter").addEventListener("input",e=>{
 const q=e.target.value.trim().toLowerCase();
 $$("#tree > li").forEach(li=>{
  let any=false;
  $$(".t-item",li).forEach(b=>{const hit=!q||b.textContent.toLowerCase().includes(q);b.parentElement.hidden=!hit;any||=hit;});
  li.hidden=!any;
  if(q&&any){$(".t-cat",li).setAttribute("aria-expanded","true");$("ul",li).hidden=false;}
 });
});

/* ---------- Abas de propriedades ---------- */
const tabs=$$(".tab");
tabs.forEach((t,i)=>{
 t.addEventListener("click",()=>activate(i));
 t.addEventListener("keydown",e=>{const d={ArrowRight:1,ArrowLeft:-1}[e.key];if(d){const n=(i+d+tabs.length)%tabs.length;activate(n);tabs[n].focus();}});
});
function activate(n){tabs.forEach((t,i)=>{t.setAttribute("aria-selected",String(i===n));t.tabIndex=i===n?0:-1;$("#"+t.getAttribute("aria-controls")).hidden=i!==n;});}

/* ---------- Passo +/- da corrente ---------- */
$$("[data-step]").forEach(b=>b.addEventListener("click",()=>{
 const inp=$("#f-in"),v=Math.max(0,parseFloat(inp.value)||0)+parseFloat(b.dataset.step);inp.value=v.toFixed(1)+" A";
}));
$("#lock-tag").addEventListener("click",e=>{const b=e.currentTarget,on=b.getAttribute("aria-pressed")!=="true";b.setAttribute("aria-pressed",String(on));$("#f-tag").readOnly=on;});
$("#f-tag").readOnly=true;

/* ---------- Simulação (ponto de ligação com o motor Rust) ---------- */
let simTimer=null;
function setSim(state){
 app.dataset.sim=state;
 const run=state==="running";
 svg.classList.toggle("running",run||state==="paused"&&svg.classList.contains("running"));
 if(state==="stopped")svg.classList.remove("running");
 const txt={running:"Simulação",paused:"Pausa",stopped:"Edição"}[state];
 $("#mode-text").textContent=txt;$("#mode-badge").dataset.mode=state==="stopped"?"edit":"sim";
 $("#sim-flag").textContent={running:"Simulação ativa",paused:"Em pausa",stopped:"Parada"}[state];
 $("#sim-state").textContent={running:"Simulação em execução",paused:"Simulador em pausa",stopped:"Simulador parado"}[state];
 $("#s-ready").textContent=state==="running"?"● Em execução":"● Pronto";
 $("#m-km1").textContent=$("#m-km2").textContent=state==="stopped"?"ABERTO":"LIGADO";
 clearInterval(simTimer);
 if(run){simTimer=setInterval(()=>{$("#m-v").textContent=(380+Math.random()*3).toFixed(1);$("#m-i").textContent=(14+Math.random()*.4).toFixed(1);},600);}
 else if(state==="stopped"){$("#m-v").textContent="0.0";$("#m-i").textContent="0.0";}
}
/* ---------- Pan / zoom do esquema (rato, toque, pinça) ---------- */
let vb={x:0,y:0,w:SHEET.w,h:SHEET.h};
const applyVB=()=>{svg.setAttribute("viewBox",`${vb.x} ${vb.y} ${vb.w} ${vb.h}`);$("#s-zoom").textContent=Math.round(SHEET.w/vb.w*100)+"%";};
const clampVB=()=>{vb.w=Math.min(Math.max(vb.w,SHEET.w/8),SHEET.w*1.5);vb.h=vb.w*SHEET.h/SHEET.w;};
function zoomAt(f,cx,cy){ // cx,cy em coordenadas da folha
 const nw=vb.w/f;const k=nw/vb.w;vb.x=cx-(cx-vb.x)*k;vb.y=cy-(cy-vb.y)*k;vb.w=nw;vb.h=nw*SHEET.h/SHEET.w;clampVB();applyVB();
}
const toSheet=(px,py)=>{const p=svg.createSVGPoint();p.x=px;p.y=py;const m=svg.getScreenCTM();return m?p.matrixTransform(m.inverse()):{x:0,y:0};};
const fit=()=>{vb={x:0,y:0,w:SHEET.w,h:SHEET.h};applyVB();};
vp.addEventListener("wheel",e=>{e.preventDefault();const p=toSheet(e.clientX,e.clientY);zoomAt(e.deltaY<0?1.12:1/1.12,p.x,p.y);},{passive:false});
const ptrs=new Map();let last=null,pinch=0;
vp.addEventListener("pointerdown",e=>{vp.setPointerCapture(e.pointerId);ptrs.set(e.pointerId,{x:e.clientX,y:e.clientY});last={x:e.clientX,y:e.clientY};vp.classList.add("panning");pinch=0;});
vp.addEventListener("pointermove",e=>{
 const p=toSheet(e.clientX,e.clientY),sn=app.dataset.snap!=="off"?16:1;
 $("#s-xy").textContent=`X: ${Math.round(p.x/sn)*sn}, Y: ${Math.round(p.y/sn)*sn}`;
 if(!ptrs.has(e.pointerId))return;
 ptrs.set(e.pointerId,{x:e.clientX,y:e.clientY});
 if(ptrs.size===2){
  const [a,b]=[...ptrs.values()],d=Math.hypot(a.x-b.x,a.y-b.y);
  if(pinch){const c=toSheet((a.x+b.x)/2,(a.y+b.y)/2);zoomAt(d/pinch,c.x,c.y);}
  pinch=d;return;
 }
 const m=svg.getScreenCTM();if(!m||!last)return;
 vb.x-=(e.clientX-last.x)/m.a;vb.y-=(e.clientY-last.y)/m.d;last={x:e.clientX,y:e.clientY};applyVB();
});
const endPtr=e=>{ptrs.delete(e.pointerId);pinch=0;last=ptrs.size?[...ptrs.values()][0]:null;if(!ptrs.size)vp.classList.remove("panning");};
vp.addEventListener("pointerup",endPtr);vp.addEventListener("pointercancel",endPtr);

/* ---------- Ações ---------- */
const actions={
 play:()=>setSim("running"),
 stop:()=>setSim("stopped"),
 pause:()=>setSim(app.dataset.sim==="paused"?"running":app.dataset.sim==="running"?"paused":app.dataset.sim),
 step:()=>{if(app.dataset.sim!=="running")setSim("paused");svg.classList.add("running");},
 reset:()=>setSim("stopped"),
 zoomin:()=>{const c=toSheet(vp.getBoundingClientRect().left+vp.clientWidth/2,vp.getBoundingClientRect().top+vp.clientHeight/2);zoomAt(1.25,c.x,c.y);},
 zoomout:()=>{const r=vp.getBoundingClientRect(),c=toSheet(r.left+r.width/2,r.top+r.height/2);zoomAt(.8,c.x,c.y);},
 fit,
 theme:()=>{const o=["system","light","dark"],cur=localStorage.getItem("rs-theme")||"system";const n=o[(o.indexOf(cur)+1)%3];setTheme(n);toast(`Tema: ${n}`);},
 grid:()=>{const off=app.dataset.snap==="off";app.dataset.snap=off?"on":"off";$$('[data-action="grid"]').forEach(b=>b.setAttribute("aria-pressed",String(off)));$("#s-snap").textContent=off?"snap ativo":"snap desligado";},
 "toggle-left":()=>app.dataset.left=app.dataset.left==="open"?"closed":"open",
 "toggle-right":()=>app.dataset.right=app.dataset.right==="open"?"closed":"open"
};
document.addEventListener("click",e=>{
 const a=e.target.closest("[data-action]");if(a&&actions[a.dataset.action])actions[a.dataset.action]();
 const t=e.target.closest("[data-tool]");if(t){$$("[data-tool]").forEach(b=>b.setAttribute("aria-pressed",String(b===t)));}
 const v=e.target.closest("[data-view]");if(v&&v.classList.contains("bn")){document.body.dataset.view=v.dataset.view;$$(".bn").forEach(b=>b.toggleAttribute("aria-current",b===v));v.setAttribute("aria-current","page");}
});
$("#panel-info").addEventListener("submit",()=>toast("Propriedades aplicadas"));
/* Atalhos */
document.addEventListener("keydown",e=>{
 if(e.target.matches("input,select,textarea"))return;
 const k=e.key.toLowerCase();
 if(k==="v")$('[data-tool="select"]').click();
 else if(k==="w")$('[data-tool="wire"]').click();
 else if(k==="t")$('[data-tool="text"]').click();
 else if(k==="g")actions.grid();
 else if(k==="0")fit();
 else if(k===" "&&!e.target.closest("button")){e.preventDefault();actions[app.dataset.sim==="running"?"stop":"play"]();}
});
let toastEl;function toast(t){if(!toastEl){toastEl=document.createElement("div");toastEl.setAttribute("role","status");toastEl.style.cssText="position:fixed;left:50%;bottom:72px;translate:-50% 0;padding:8px 14px;background:var(--s3);border:1px solid var(--border-strong);border-radius:6px;font:600 12px var(--font-mono);z-index:99;transition:opacity .2s";document.body.appendChild(toastEl);}toastEl.textContent=t;toastEl.style.opacity=1;clearTimeout(toastEl._t);toastEl._t=setTimeout(()=>toastEl.style.opacity=0,1400);}

buildTree();applyVB();setSim("stopped");
})();
