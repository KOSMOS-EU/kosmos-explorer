import React, {useState, useEffect} from 'react';
import {useSelector, useDispatch} from 'react-redux';
import {Icon, Image, ToolBar} from '../../../utils/general';
import {TauriToolBar} from '../../../components/TauriToolBar';
import {dispatchAction, handleFileOpen} from '../../../actions';
import {addCloud, getUser, listSpaces, listFiles, oidcLogin, updateToken} from '../../../utils/cloudApi';
import './assets/fileexpo.scss';

const NavTitle = (props)=>{
  var src = props.icon || "folder"

  return(
    <div className="navtitle flex prtclk" data-action={props.action}
      data-payload={props.payload} onClick={dispatchAction}>
      <Icon className="mr-1" src={"win/"+src+"-sm"} width={props.isize || 16}/>
      <span>{props.title}</span>
    </div>
  )
}

const FolderDrop = ({dir})=>{
  const files = useSelector(state => state.files);
  const folder = files.data.getId(dir);

  return (
    <>
      {folder.data && folder.data.map((item,i) => {
        if(item.type=="folder"){
          return <Dropdown key={i} icon={item.info && item.info.icon}
            title={item.name} notoggle={item.data.length==0} dir={item.id}/>
        }
      })}
    </>
  )
}

const Dropdown = (props)=>{
  const [open, setOpen] = useState(props.isDropped!=null);
  const special = useSelector(state => state.files.data.special);
  const [fid, setFID] = useState(()=>{
    if(props.payload) return props.payload
    if(props.spid) return special[props.spid]
    else return props.dir
  })
  const toggle = ()=> setOpen(!open)

  return (
    <div className="dropdownmenu">
      <div className="droptitle">
        {!props.notoggle?(
          <Icon className="arrUi" fafa={open?"faChevronDown":"faChevronRight"}
            width={10} onClick={toggle} pr/>
        ):<Icon className="arrUi opacity-0" fafa="faCircle" width={10}/>}
        <NavTitle icon={props.icon} title={props.title} isize={props.isize}
          action={props.action!=""?(props.action || "FILEDIR"):null} payload={fid}/>
        {props.pinned!=null?(
          <Icon className="pinUi" src="win/pinned" width={16}/>
        ):null}
      </div>
      {!props.notoggle?(
        <div className="dropcontent">
          {open?props.children:null}
          {open&&fid!=null?<FolderDrop dir={fid}/>:null}
        </div>
      ):null}
    </div>
  )
}

const CloudDialogOverlay = ({onClose})=>(
  <div onClick={onClose} style={{
    position:'absolute', inset:0, zIndex:100,
    background:'rgba(0,0,0,0.3)'
  }}/>
)

const CloudDialogContent = ({onClose, onSave})=>{
  const [name, setName] = useState('');
  const [url, setUrl] = useState('');

  const handleSave = ()=>{
    if(!name.trim() || !url.trim()) return;
    onSave({name: name.trim(), url: url.trim()});
  }

  return (
    <div style={{
      position:'absolute', zIndex:101,
      top:'50%', left:'50%', transform:'translate(-50%,-50%)',
      background:'var(--bg0)', borderRadius:8, width:380,
      boxShadow:'0 8px 32px rgba(0,0,0,0.2)'
    }}>
      <div className="toolbar" style={{borderRadius:'8px 8px 0 0', padding:'0 12px'}}>
        <div className="topInfo flex flex-grow items-center">
          <div className="appFullName text-xss">Neue Cloud</div>
        </div>
        <div className="actbtns flex items-center">
          <Icon className="closeBtn" src="close" ui width={8} onClick={onClose}
            style={{height:'100%',padding:'0 14px'}}/>
        </div>
      </div>
      <div style={{padding:'16px 20px', display:'flex', flexDirection:'column', gap:10}}>
        <div>
          <div className="text-xss" style={{marginBottom:4, color:'var(--txt-col)'}}>Name</div>
          <input className="path-field" value={name} onChange={e=>setName(e.target.value)}
            placeholder="z.B. PartheCloud Brandis"
            style={{width:'100%', padding:'6px 8px', borderRadius:4,
              border:'1px solid var(--gray-txt)', background:'var(--bg1)',
              color:'var(--txt-col)'}}/>
        </div>
        <div>
          <div className="text-xss" style={{marginBottom:4, color:'var(--txt-col)'}}>Server-URL</div>
          <input className="path-field" value={url} onChange={e=>setUrl(e.target.value)}
            placeholder="https://cloud.example.org"
            style={{width:'100%', padding:'6px 8px', borderRadius:4,
              border:'1px solid var(--gray-txt)', background:'var(--bg1)',
              color:'var(--txt-col)'}}/>
        </div>
      </div>
      <div style={{padding:'10px 20px 16px', display:'flex', justifyContent:'flex-end', gap:8,
        borderTop:'1px solid var(--gray-txt)'}}>
        <div className="drdwcont flex prtclk" onClick={onClose}
          style={{padding:'5px 16px', borderRadius:4, border:'1px solid var(--gray-txt)',
            cursor:'default', fontSize:13, color:'var(--txt-col)'}}>
          Abbrechen
        </div>
        <div className="drdwcont flex prtclk" onClick={handleSave}
          style={{padding:'5px 16px', borderRadius:4, background:'#0067c0',
            color:'#ffffff', cursor:'default', fontSize:13, fontWeight:600}}>
          Hinzufügen
        </div>
      </div>
    </div>
  )
}

export const Explorer = (props)=>{
  const apps = useSelector(state => state.apps);
  const wnapp = useSelector(state => state.apps.explorer);
  const files = useSelector(state => state.files);
  const clouds = useSelector(state => state.clouds);
  const fdata = files.data.getId(files.cdir);
  const [cpath, setPath] = useState(files.cpath);
  const [searchtxt, setShText] = useState("")
  const dispatch = useDispatch();

  const handleChange = (e) => setPath(e.target.value)
  const handleSearchChange = (e) => setShText(e.target.value)

  const handleEnter = (e)=>{
    if(e.key==="Enter"){
      dispatch({type: "FILEPATH", payload: cpath})
    }
  }

  const DirCont = ()=>{
    var arr = [], curr = fdata,index=0;
    
    while(curr){
      arr.push(
        <div key={index++} className="dirCont flex items-center">
          <div className="dncont" onClick={dispatchAction} tabIndex="-1"
            data-action="FILEDIR" data-payload={curr.id}>{curr.name}</div>
          <Icon className="dirchev" fafa="faChevronRight" width={8}/>
        </div>
      )

      curr = curr.host
    }

    arr.push(
      <div key={index++} className="dirCont flex items-center">
        <div className="dncont" tabIndex="-1">This PC</div>
        <Icon className="dirchev" fafa="faChevronRight" width={8}/>
      </div>
    )

    arr.push(
      <div key={index++} className="dirCont flex items-center">
        <Icon className="pr-1 pb-px" src={"win/" + fdata.info.icon + "-sm"} width={16}/>
        <Icon className="dirchev" fafa="faChevronRight" width={8}/>
      </div>
    )

    return (
      <div key={index++} className="dirfbox h-full flex">
        {arr.reverse()}
      </div>
    )
  }

  useEffect(()=>{
    setPath(files.cpath)
    setShText("")
  }, [files.cpath])

  const isStandalone = props.standalone;

  if (isStandalone) {
    return (
      <div className="msfiles" style={{
        position: 'fixed', inset: 0, borderRadius: 0,
        display: 'flex', flexDirection: 'column',
        background: 'var(--bg0)'
      }}>
        <TauriToolBar icon="explorer" name="KOSMOS Explorer"/>
        {clouds.dialogOpen && <CloudDialogContent
          onClose={()=>dispatch({type:'CLOUD_DIALOG_CLOSE'})}
          onSave={async (data)=>{
            const list = await addCloud(data.name, data.url);
            dispatch({type:'CLOUD_SET_LIST', payload: list});
            dispatch({type:'CLOUD_DIALOG_CLOSE'});
          }}
        />}
        <div className="windowScreen flex flex-col" style={{flex: 1}}>
          <Ribbon onNew={()=>dispatch({type:'CLOUD_DIALOG_OPEN'})}/>
          <div className="restWindow flex-grow flex flex-col">
            <div className="sec1">
              <Icon className={"navIcon hvtheme" + (files.hid==0?" disableIt":"")}
                fafa="faArrowLeft" width={14} click="FILEPREV" pr/>
              <Icon className={"navIcon hvtheme" + ((files.hid+1)==files.hist.length?" disableIt":"")}
                fafa="faArrowRight" width={14} click="FILENEXT" pr/>
              <Icon className="navIcon hvtheme" fafa="faArrowUp" width={14} click="FILEBACK" pr/>
              <div className="path-bar noscroll" tabIndex="-1">
                <input className="path-field" type="text" value={cpath}
                  onChange={handleChange} onKeyDown={handleEnter}/>
                <DirCont/>
              </div>
              <div className="srchbar">
                <Icon className="searchIcon" src="search" width={12}/>
                <input type="text" onChange={handleSearchChange} value={searchtxt} placeholder='Suchen'/>
              </div>
            </div>
            <div className="sec2">
              <NavPane/>
              <NavResize/>
              <CloudView/>
            </div>
            <StatusBar files={files} fdata={fdata}/>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="msfiles floatTab dpShad" data-size={wnapp.size}
      data-max={wnapp.max} style={{
        ...(wnapp.size=="cstm"?wnapp.dim:null),
        zIndex: wnapp.z
      }} data-hide={wnapp.hide} id={wnapp.icon+"App"}>
      <ToolBar app={wnapp.action} icon={wnapp.icon} size={wnapp.size}
        name="File Explorer"/>
      <div className="windowScreen flex flex-col">
        <Ribbon/>
        <div className="restWindow flex-grow flex flex-col">
          <div className="sec1">
            <Icon className={"navIcon hvtheme" + (files.hid==0?" disableIt":"")}
              fafa="faArrowLeft" width={14} click="FILEPREV" pr/>
            <Icon className={"navIcon hvtheme" + ((files.hid+1)==files.hist.length?" disableIt":"")}
              fafa="faArrowRight" width={14} click="FILENEXT" pr/>
            <Icon className="navIcon hvtheme" fafa="faArrowUp" width={14} click="FILEBACK" pr/>
            <div className="path-bar noscroll" tabIndex="-1">
              <input className="path-field" type="text" value={cpath}
                onChange={handleChange} onKeyDown={handleEnter}/>
              <DirCont/>
            </div>
            <div className="srchbar">
              <Icon className="searchIcon" src="search" width={12}/>
              <input type="text" onChange={handleSearchChange} value={searchtxt} placeholder='Search'/>
            </div>
          </div>
          <div className="sec2">
            <NavPane/>
            <ContentArea searchtxt={searchtxt}/>
          </div>
          <div className="sec3">
            <div className="item-count text-xs">{fdata.data.length} items</div>
            <div className="view-opts flex">
              <Icon className="viewicon hvtheme p-1" click="FILEVIEW" payload="5" open={files.view==5}
                src="win/viewinfo" width={16}/>
              <Icon className="viewicon hvtheme p-1" click="FILEVIEW" payload="1" open={files.view==1}
                src="win/viewlarge" width={16}/>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

const ContentArea = ({searchtxt})=>{
  const files = useSelector(state => state.files);
  const special = useSelector(state => state.files.data.special);
  const [selected, setSelect] = useState(null);
  const fdata = files.data.getId(files.cdir);
  const dispatch = useDispatch();

  const handleClick = (e)=>{
    e.stopPropagation();
    setSelect(e.target.dataset.id);
  }

  const handleDouble = (e)=>{
    e.stopPropagation();
    handleFileOpen(e.target.dataset.id);
  }

  const emptyClick = (e)=>{
    setSelect(null);
  }

  const handleKey = (e)=>{
    if(e.key == "Backspace"){
      dispatch({type: "FILEPREV"})
    }
  }

  return(
    <div className="contentarea" onClick={emptyClick} onKeyDown={handleKey} tabIndex="-1">
      <div className="contentwrap win11Scroll">
        <div className="gridshow" data-size="lg">
          {fdata.data.map((item,i)=>{
            return item.name.includes(searchtxt) && (
              <div key={i} className="conticon hvtheme flex flex-col items-center prtclk"
                data-id={item.id} data-focus={selected==item.id}
                onClick={handleClick} onDoubleClick={handleDouble}>
                <Image src={`icon/win/${item.info.icon}`}/>
                <span>{item.name}</span>
              </div>
            )
          })}
        </div>
        {fdata.data.length==0?(
          <span className="text-xs mx-auto">This folder is empty.</span>
        ):null}
      </div>
    </div>
  )
}

const NavResize = ()=>{
  const startDrag = (e)=>{
    e.preventDefault();
    e.stopPropagation();
    const sec2 = e.target.closest('.sec2');
    const navpane = sec2?.querySelector('.navpane');
    if(!navpane) return;

    const startX = e.clientX;
    const startW = navpane.offsetWidth;

    const onMove = (ev)=>{
      ev.preventDefault();
      const w = Math.max(120, Math.min(startW + ev.clientX - startX, sec2.offsetWidth * 0.5));
      navpane.style.width = w + 'px';
    };
    const onUp = ()=>{
      document.removeEventListener('mousemove', onMove);
      document.removeEventListener('mouseup', onUp);
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
    };
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';
    document.addEventListener('mousemove', onMove);
    document.addEventListener('mouseup', onUp);
  };

  return <div className="nav-resize" onMouseDown={startDrag}/>;
}

const CloudView = ()=>{
  const clouds = useSelector(state => state.clouds);
  const activeCloud = clouds.activeCloud !== null ? clouds.list[clouds.activeCloud] : null;
  const activeSpace = clouds.activeSpace;

  if(!activeCloud || !activeCloud.connected){
    return (
      <div className="contentarea" style={{display:'flex', alignItems:'center', justifyContent:'center'}}>
        <span className="text-xs">Cloud auswählen</span>
      </div>
    )
  }

  // Build the cloud URL for the active space
  let cloudUrl = activeCloud.url.replace(/\/$/, '');
  if(activeSpace){
    cloudUrl += '/f/' + activeSpace;
  }
  // TODO: append compact/theme params when OpenCloud supports them
  // cloudUrl += '?appTheme=win&appCompact=true';

  return (
    <div className="contentarea" style={{padding:0, overflow:'hidden'}}>
      <iframe
        src={cloudUrl}
        style={{width:'100%', height:'100%', border:'none'}}
        title="Cloud"
      />
    </div>
  )
}

const FolderNode = ({cloud, space, path, name, icon, dispatch})=>{
  const [open, setOpen] = useState(false);
  const [children, setChildren] = useState(null);

  const toggle = async ()=>{
    if(!open && children === null){
      try {
        const items = await listFiles(cloud.url, cloud.bearer, space.id, path);
        setChildren(items.filter(f => f.isFolder));
      } catch { setChildren([]); }
    }
    setOpen(!open);
  }

  const hasChildren = children === null || children.length > 0;

  return (
    <div className="dropdownmenu" style={{paddingLeft:'0.6em'}}>
      <div className="droptitle">
        {hasChildren ? (
          <Icon className="arrUi" fafa={open ? "faChevronDown" : "faChevronRight"}
            width={10} onClick={toggle} pr/>
        ) : (
          <Icon className="arrUi opacity-0" fafa="faCircle" width={10}/>
        )}
        <div className="navtitle flex prtclk" onClick={()=>{
          dispatch({type: 'CLOUD_SELECT_SPACE', payload: space.id});
          dispatch({type: 'CLOUD_NAVIGATE', payload: path});
        }}>
          <Icon className="mr-1" src={"win/"+(icon || 'folder')+"-sm"} width={16}/>
          <span>{name}</span>
        </div>
      </div>
      {open && children && children.length > 0 && (
        <div className="dropcontent">
          {children.map((child) => (
            <FolderNode key={child.id} cloud={cloud} space={space}
              path={path === '/' ? '/' + child.name : path + '/' + child.name}
              name={child.name} icon="folder" dispatch={dispatch}/>
          ))}
        </div>
      )}
    </div>
  )
}

const SpaceNode = ({cloud, cloudIndex, space, isActive, dispatch})=>{
  const [open, setOpen] = useState(false);
  const [children, setChildren] = useState(null);

  const toggle = async ()=>{
    if(!open && children === null){
      try {
        const items = await listFiles(cloud.url, cloud.bearer, space.id, '/');
        setChildren(items.filter(f => f.isFolder));
      } catch { setChildren([]); }
    }
    setOpen(!open);
  }

  const select = ()=>{
    dispatch({type: 'CLOUD_SELECT_SPACE', payload: space.id});
    dispatch({type: 'CLOUD_NAVIGATE', payload: '/'});
  }

  const hasChildren = children === null || children.length > 0;
  const spaceIcon = space.driveType === 'personal' ? 'user' : 'folder';

  return (
    <div className="dropdownmenu" style={{paddingLeft:'0.6em'}}>
      <div className="droptitle">
        {hasChildren ? (
          <Icon className="arrUi" fafa={open ? "faChevronDown" : "faChevronRight"}
            width={10} onClick={toggle} pr/>
        ) : (
          <Icon className="arrUi opacity-0" fafa="faCircle" width={10}/>
        )}
        <div className={"navtitle flex prtclk" + (isActive ? " font-bold" : "")}
          onClick={select}>
          <Icon className="mr-1" src={"win/"+spaceIcon+"-sm"} width={16}/>
          <span>{space.name}</span>
        </div>
      </div>
      {open && children && children.length > 0 && (
        <div className="dropcontent">
          {children.map((child) => (
            <FolderNode key={child.id} cloud={cloud} space={space}
              path={'/' + child.name} name={child.name} icon="folder" dispatch={dispatch}/>
          ))}
        </div>
      )}
    </div>
  )
}

const NavPane = ({})=>{
  const clouds = useSelector(state => state.clouds);
  const files = useSelector(state => state.files);
  const dispatch = useDispatch();

  const connectCloud = async (ci)=>{
    const cloud = clouds.list[ci];
    if(!cloud) return;

    let bearer = cloud.bearer;

    // If we have a token, try it first
    if(bearer) {
      try {
        const user = await getUser(cloud.url, bearer);
        const spaces = await listSpaces(cloud.url, bearer);
        dispatch({type: 'CLOUD_CONNECTED', payload: {index: ci, user, spaces, bearer}});
        setOpenClouds(prev => ({...prev, [ci]: true}));
        return;
      } catch(err) {
        // Bearer expired or invalid, fall through to OIDC
      }
    }

    // No bearer or expired → OIDC login
    try {
      bearer = await oidcLogin(cloud.url);
      await updateToken(ci, bearer);

      const user = await getUser(cloud.url, bearer);
      const spaces = await listSpaces(cloud.url, bearer);
      dispatch({type: 'CLOUD_CONNECTED', payload: {index: ci, user, spaces, bearer}});
      setOpenClouds(prev => ({...prev, [ci]: true}));
    } catch(err) {
      alert('Anmeldung fehlgeschlagen: ' + err);
    }
  }

  const selectSpace = (ci, space)=>{
    // Just set the active space — the CloudView iframe navigates automatically
    dispatch({type: 'CLOUD_SELECT_SPACE', payload: space.id});
  }

  const [openClouds, setOpenClouds] = useState({});
  const toggleCloud = (ci) => setOpenClouds(prev => ({...prev, [ci]: !prev[ci]}));

  const disconnectCloud = async (ci) => {
    // Bearer ungültig machen — auch in der gespeicherten Config
    await updateToken(ci, '');
    dispatch({type: 'CLOUD_DISCONNECTED', payload: ci});
    setOpenClouds(prev => ({...prev, [ci]: false}));
  }

  const removeCloud = async (ci) => {
    const {removeCloud: apiRemove} = await import('../../../utils/cloudApi');
    const list = await apiRemove(ci);
    dispatch({type: 'CLOUD_SET_LIST', payload: list});
  }

  // Handle context menu actions via Redux pendingAction
  useEffect(()=>{
    if(!clouds.pendingAction) return;
    const {type, index} = clouds.pendingAction;
    dispatch({type: 'CLOUD_PENDING_HANDLED'});
    if(type === 'CLOUD_CONNECT_CTX') connectCloud(index);
    else if(type === 'CLOUD_DISCONNECT_CTX') disconnectCloud(index);
    else if(type === 'CLOUD_REMOVE_CTX') removeCloud(index);
  }, [clouds.pendingAction]);

  return (
    <div className="navpane win11Scroll">
      <div className="extcont">
        {clouds.list.map((cloud, ci) => {
          const isOpen = cloud.connected && (openClouds[ci] !== undefined ? openClouds[ci] : true);
          return (
            <div key={ci} className="dropdownmenu" data-menu="cloud" data-payload={ci}>
              <div className="droptitle">
                <Icon className="arrUi" fafa={isOpen ? "faChevronDown" : "faChevronRight"}
                  width={10} onClick={(e)=>{
                    if(e.button !== 0) return; // ignore right-click
                    if(!cloud.connected) connectCloud(ci);
                    else toggleCloud(ci);
                  }} pr/>
                <div className="navtitle flex prtclk" onClick={(e)=>{
                  if(e.button !== 0) return;
                  if(!cloud.connected) connectCloud(ci);
                  else toggleCloud(ci);
                }}>
                  <Icon className="mr-1" src={"win/"+(cloud.connected ? "onedrive" : "disc")+"-sm"} width={16}/>
                  <span>{cloud.name}</span>
                </div>
              </div>
              {isOpen && cloud.connected && cloud.spaces && (
                <div className="dropcontent">
                  {cloud.spaces.map((space) => (
                    <SpaceNode key={space.id} cloud={cloud} cloudIndex={ci} space={space}
                      isActive={clouds.activeSpace === space.id} dispatch={dispatch}/>
                  ))}
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  )
}

const StatusBar = ({files, fdata})=>{
  const clouds = useSelector(state => state.clouds);
  const online = clouds.list.filter(c => c.connected).length;
  const total = clouds.list.length;
  const active = clouds.activeCloud !== null ? clouds.list[clouds.activeCloud] : null;

  return (
    <div className="sec3">
      <div className="item-count text-xs">{fdata.data.length} Elemente</div>
      <div className="text-xs flex items-center" style={{gap: 16, marginLeft: 'auto', marginRight: 8}}>
        {active && active.connected && (
          <span>🔒 {active.name} · {active.user}</span>
        )}
        <span>Online: {online}/{total}</span>
      </div>
      <div className="view-opts flex">
        <Icon className="viewicon hvtheme p-1" click="FILEVIEW" payload="5" open={files.view==5}
          src="win/viewinfo" width={16}/>
        <Icon className="viewicon hvtheme p-1" click="FILEVIEW" payload="1" open={files.view==1}
          src="win/viewlarge" width={16}/>
      </div>
    </div>
  )
}

const Ribbon = ({onNew})=>{
  return (
    <div className="msribbon flex">
      <div className="ribsec">
        <div className="drdwcont flex prtclk" onClick={onNew} style={{cursor:'default'}}>
          <Icon src="new" ui width={18} margin="0 6px"/>
          <span>Neu</span>
        </div>
      </div>
      <div className="ribsec">
        <Icon src="cut" ui width={18} margin="0 6px"/>
        <Icon src="copy" ui width={18} margin="0 6px"/>
        <Icon src="paste" ui width={18} margin="0 6px"/>
        <Icon src="rename" ui width={18} margin="0 6px"/>
      </div>
      <div className="ribsec">
        <div className="drdwcont flex">
          <Icon src="sort" ui width={18} margin="0 6px"/>
          <span>Sort</span>
        </div>
        <div className="drdwcont flex">
          <Icon src="view" ui width={18} margin="0 6px"/>
          <span>View</span>
        </div>
      </div>
    </div>
  )
}
