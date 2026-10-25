// Roci UI kit v2 · donut.js · the Option B donut (25-oct-2026)
// Inner ring: value (lane colour) against waste (ring background). Outer thin ring: the waste by cause.
// Throttled is drawn as a dashed gap outside the ring (nobody was working). Centre: cost (large) or value share (small).
// Inputs: d = {value, <cause keys...>, cost} in minutes; WASTE = [{k, name, c (css colour), fill?}]; valueSlices = [{v, fill}].
// Colours come from theme.css tokens: lanes --l-*, causes --w-*, ring --ring-bg.
const WASTE=[{k:'human',name:'waiting on human',c:'var(--w-human)'},{k:'valid',name:'waiting on validation',c:'var(--w-valid)'},{k:'throttled',name:'throttled',c:'var(--w-throttled)'},{k:'rework',name:'rework',c:'var(--w-rework)'},{k:'retry',name:'retry',c:'var(--w-retry)'},{k:'idle',name:'idle',c:'var(--w-idle)'},{k:'overhead',name:'build overhead',c:'var(--w-overhead)'}];
const total=d=>d.value+WASTE.reduce((s,w)=>s+d[w.k],0);
const waste=d=>WASTE.reduce((s,w)=>s+d[w.k],0);
const pct=d=>total(d)?Math.round(100*d.value/total(d)):0;
const r1=v=>Math.round(v*10)/10;
const NS='http://www.w3.org/2000/svg';
const el=(t,a,parent)=>{const e=document.createElementNS(NS,t);for(const k in a)e.setAttribute(k,a[k]);if(parent)parent.appendChild(e);return e;};

const txt=(svg,x,y,t,cls,size,anchor)=>{const e=el('text',{x,y,'text-anchor':anchor||'middle','font-size':size||12,class:cls||''},svg);e.textContent=t;return e;};

function arc(cx,cy,r,a0,a1){const big=a1-a0>Math.PI?1:0;return {x0:cx+r*Math.cos(a0),y0:cy+r*Math.sin(a0),x1:cx+r*Math.cos(a1),y1:cy+r*Math.sin(a1),big};}

function ring(svg,cx,cy,r,w,slices){
  const sum=slices.reduce((s,x)=>s+x.v,0);if(!sum)return;let a=-Math.PI/2;
  slices.forEach(s=>{if(s.v<=0)return;const a1=a+2*Math.PI*s.v/sum;const o=arc(cx,cy,r,a,a1),i=arc(cx,cy,r-w,a,a1);
    const d=`M${o.x0} ${o.y0}A${r} ${r} 0 ${o.big} 1 ${o.x1} ${o.y1}L${i.x1} ${i.y1}A${r-w} ${r-w} 0 ${o.big} 0 ${i.x0} ${i.y0}Z`;
    if(s.under)el('path',{d,fill:s.under},svg);
    el('path',{d,fill:s.fill,stroke:'var(--surface)','stroke-width':'1.2'},svg);a=a1;});
}

function donutB(svg,d,valueSlices,small,onCause){
  if(!total(d)){el('circle',{cx:60,cy:60,r:46,fill:'none',stroke:'var(--ring-bg)','stroke-width':small?10:16,'stroke-dasharray':'3 5'},svg);if(!small)txt(svg,60,64,'not started','mut',10);return;}
  ring(svg,60,60,46,small?14:16,valueSlices.concat([{v:waste(d),fill:'var(--ring-bg)'}]));
  const sum=total(d);let a=-Math.PI/2+2*Math.PI*d.value/sum;
  WASTE.forEach(w=>{const v=d[w.k];if(!v)return;const a1=a+2*Math.PI*v/sum;let p;
    if(w.k==='throttled'){const o=arc(60,60,54,a,a1);p=el('path',{d:`M${o.x0} ${o.y0}A54 54 0 ${o.big} 1 ${o.x1} ${o.y1}`,fill:'none',stroke:'var(--w-throttled)','stroke-width':small?2:1,'stroke-dasharray':'2 3'},svg);}
    else{const o=arc(60,60,58,a,a1),i=arc(60,60,50,a,a1);const dd=`M${o.x0} ${o.y0}A58 58 0 ${o.big} 1 ${o.x1} ${o.y1}L${i.x1} ${i.y1}A50 50 0 ${o.big} 0 ${i.x0} ${i.y0}Z`;if(w.fill)el('path',{d:dd,fill:'var(--surface)'},svg);p=el('path',{d:dd,fill:w.fill||w.c,stroke:'var(--surface)','stroke-width':1},svg);}
    const t=el('title',{},p);t.textContent=w.name+' · '+v+' min';
    if(onCause)p.addEventListener('click',e=>{e.stopPropagation();onCause(w.k);});
    a=a1;});
  if(small){txt(svg,60,67,pct(d)+'%','m',20);}
  else{txt(svg,60,56,r1(d.cost),'m',20).setAttribute('font-weight','500');txt(svg,60,70,'USD-eq','mut',10);}
}

// usage: const svg=el('svg',{viewBox:'0 0 120 120'},container); donutB(svg,d,[{v:d.value,fill:'var(--l-build)'}],false,k=>select(k));
export {el,txt,arc,ring,donutB,total,waste,pct,WASTE};
