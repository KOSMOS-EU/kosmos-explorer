import {Bin} from '../utils/bin';
import fdata from './dir.json';

const defState = {
  cdir: "%clouds%",
  hist: [],
  hid: 0,
  view: 1
};

defState.hist.push(defState.cdir);
defState.data = new Bin();
defState.data.parse(fdata);

const fileReducer = (state = defState, action) => {
  var tmp = {...state};
  var navHist = false;

  if (action.type === "FILEDIR") {
    tmp.cdir = action.payload
  }else if (action.type === "FILEPATH") {
    var pathid = tmp.data.parsePath(action.payload);
    if(pathid) tmp.cdir = pathid;
  }else if (action.type === "FILEBACK") {
    var item = tmp.data.getId(tmp.cdir);
    if(item && item.host){
      tmp.cdir = item.host.id;
    }
  }else if(action.type === "FILEVIEW"){
    tmp.view = action.payload;
  }else if(action.type === "FILEPREV"){
    tmp.hid--;
    if(tmp.hid<0) tmp.hid = 0;
    navHist = true;
  }else if(action.type === "FILENEXT"){
    tmp.hid++;
    if(tmp.hid>tmp.hist.length-1) tmp.hid = tmp.hist.length-1;
    navHist = true;
  }else if(action.type === "CLOUD_INJECT_FILES"){
    // Inject cloud files into the Bin tree
    // payload: { parentId, files: [{name, type, info, data}] }
    var parent = tmp.data.getId(action.payload.parentId);
    if(parent){
      var items = [];
      for(var f of action.payload.files){
        var item = tmp.data.parseFolder(f, f.name, parent);
        items.push(item);
      }
      parent.setData(items);
    }
  }else if(action.type === "CLOUD_INJECT_SPACE"){
    // Add a space folder under the clouds root
    // payload: { name, spaceId, icon }
    var cloudsId = tmp.data.special["%clouds%"];
    var cloudsFolder = tmp.data.getId(cloudsId);
    if(cloudsFolder){
      var spaceData = {
        type: "folder",
        name: action.payload.name,
        info: { icon: action.payload.icon || "folder", spid: action.payload.spaceId }
      };
      var spaceItem = tmp.data.parseFolder(spaceData, action.payload.name, cloudsFolder);
      cloudsFolder.data.push(spaceItem);
    }
  }

  if(!navHist && tmp.cdir!=tmp.hist[tmp.hid]){
    tmp.hist.splice(tmp.hid+1);
    tmp.hist.push(tmp.cdir);
    tmp.hid = tmp.hist.length - 1;
  }

  tmp.cdir = tmp.hist[tmp.hid];
  if(tmp.cdir && tmp.cdir.includes && tmp.cdir.includes("%")){
    if(tmp.data.special[tmp.cdir]!=null){
      tmp.cdir = tmp.data.special[tmp.cdir];
      tmp[tmp.hid] = tmp.cdir;
    }
  }

  tmp.cpath = tmp.data.getPath(tmp.cdir);
  return tmp
}

export default fileReducer;
