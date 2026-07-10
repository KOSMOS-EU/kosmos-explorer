import React from 'react';
import {Icon} from '../utils/general';

let tauriWindow = null;

async function getTauriWindow() {
  if (tauriWindow) return tauriWindow;
  try {
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    tauriWindow = getCurrentWindow();
    return tauriWindow;
  } catch {
    return null; // Running in browser, not Tauri
  }
}

async function minimize() { const w = await getTauriWindow(); w?.minimize(); }
async function toggleMaximize() { const w = await getTauriWindow(); w?.toggleMaximize(); }
async function close() { const w = await getTauriWindow(); w?.close(); }
async function startDrag(e) {
  if (e.button !== 0) return;
  if (e.detail === 2) { toggleMaximize(); return; }
  const w = await getTauriWindow();
  w?.startDragging();
}

export const TauriToolBar = ({icon, name}) => {
  return (
    <div className="toolbar" style={{borderRadius: 0}}>
      <div className="topInfo flex flex-grow items-center"
        onMouseDown={startDrag} style={{cursor: 'default'}}>
        <Icon src={icon} width={14}/>
        <div className="appFullName text-xss">{name}</div>
      </div>
      <div className="actbtns flex items-center">
        <div className="uicon" style={{height:'100%',padding:'0 14px',display:'flex',alignItems:'center'}}
          onClick={minimize}>
          <Icon src="minimize" ui width={8}/>
        </div>
        <div className="uicon" style={{height:'100%',padding:'0 14px',display:'flex',alignItems:'center'}}
          onClick={toggleMaximize}>
          <Icon src="maximize" ui width={8}/>
        </div>
        <div className="uicon closeBtn" style={{height:'100%',padding:'0 14px',display:'flex',alignItems:'center'}}
          onClick={close}>
          <Icon src="close" ui width={8}/>
        </div>
      </div>
    </div>
  );
};
