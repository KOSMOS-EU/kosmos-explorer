// Cloud state management
// Clouds are persisted in localStorage, files are fetched from API

// Cloud persistence is handled by Rust backend (cloud_load, cloud_add, etc.)
// Redux store holds the runtime state; Rust holds the persisted config.

const defState = {
  list: [],        // [{name, url, token?, connected?, user?, spaces?}]
  activeCloud: null,         // index into list
  activeSpace: null,         // space id
  cdir: null,                // current directory path (/ or /subdir/...)
  files: [],                 // current file listing
  hist: [],                  // navigation history
  hid: -1,                   // history index
  view: 'settings',          // 'explorer' | 'settings'
  loading: false,
  error: null,
  dialogOpen: false,
};

const cloudReducer = (state = defState, action) => {
  const tmp = { ...state };

  switch (action.type) {
    case 'CLOUD_SET_LIST': {
      tmp.list = action.payload;
      break;
    }
    case 'CLOUD_DIALOG_OPEN': {
      tmp.dialogOpen = true;
      break;
    }
    case 'CLOUD_DIALOG_CLOSE': {
      tmp.dialogOpen = false;
      break;
    }
    case 'CLOUD_CONNECT_CTX':
    case 'CLOUD_DISCONNECT_CTX':
    case 'CLOUD_REMOVE_CTX': {
      // These are handled by NavPane via pendingAction
      tmp.pendingAction = { type: action.type, index: parseInt(action.payload) };
      break;
    }
    case 'CLOUD_PENDING_HANDLED': {
      tmp.pendingAction = null;
      break;
    }
    case 'CLOUD_ADD': {
      const cloud = { name: action.payload.name, url: action.payload.url, connected: false };
      tmp.list = [...tmp.list, cloud];

      break;
    }
    case 'CLOUD_REMOVE': {
      tmp.list = tmp.list.filter((_, i) => i !== action.payload);
      if (tmp.activeCloud === action.payload) {
        tmp.activeCloud = null;
        tmp.activeSpace = null;
        tmp.files = [];
        tmp.view = 'settings';
      }

      break;
    }
    case 'CLOUD_UPDATE': {
      const { index, ...updates } = action.payload;
      tmp.list = tmp.list.map((c, i) => i === index ? { ...c, ...updates } : c);

      break;
    }
    case 'CLOUD_CONNECTED': {
      const { index, user, spaces, bearer } = action.payload;
      tmp.list = tmp.list.map((c, i) => i === index
        ? { ...c, connected: true, user, spaces, bearer }
        : c);
      tmp.activeCloud = index;
      if (spaces && spaces.length > 0) {
        const personal = spaces.find(s => s.driveType === 'personal');
        tmp.activeSpace = personal ? personal.id : spaces[0].id;
      }
      tmp.cdir = '/';
      tmp.view = 'explorer';
      tmp.hist = [{ space: tmp.activeSpace, path: '/' }];
      tmp.hid = 0;

      break;
    }
    case 'CLOUD_DISCONNECTED': {
      tmp.list = tmp.list.map((c, i) => i === action.payload
        ? { ...c, connected: false, user: null, spaces: null, bearer: null }
        : c);
      if (tmp.activeCloud === action.payload) {
        tmp.activeCloud = null;
        tmp.activeSpace = null;
        tmp.files = [];
        tmp.view = 'settings';
      }

      break;
    }
    case 'CLOUD_SELECT_SPACE': {
      tmp.activeSpace = action.payload;
      tmp.cdir = '/';
      tmp.files = [];
      tmp.hist = [...tmp.hist.slice(0, tmp.hid + 1), { space: action.payload, path: '/' }];
      tmp.hid = tmp.hist.length - 1;
      break;
    }
    case 'CLOUD_NAVIGATE': {
      tmp.cdir = action.payload;
      tmp.files = [];
      tmp.hist = [...tmp.hist.slice(0, tmp.hid + 1), { space: tmp.activeSpace, path: action.payload }];
      tmp.hid = tmp.hist.length - 1;
      break;
    }
    case 'CLOUD_BACK': {
      if (tmp.cdir === '/') break;
      const parts = tmp.cdir.split('/').filter(Boolean);
      parts.pop();
      tmp.cdir = parts.length > 0 ? '/' + parts.join('/') : '/';
      tmp.hist = [...tmp.hist.slice(0, tmp.hid + 1), { space: tmp.activeSpace, path: tmp.cdir }];
      tmp.hid = tmp.hist.length - 1;
      break;
    }
    case 'CLOUD_HIST_PREV': {
      if (tmp.hid > 0) {
        tmp.hid--;
        const entry = tmp.hist[tmp.hid];
        tmp.activeSpace = entry.space;
        tmp.cdir = entry.path;
        tmp.files = [];
      }
      break;
    }
    case 'CLOUD_HIST_NEXT': {
      if (tmp.hid < tmp.hist.length - 1) {
        tmp.hid++;
        const entry = tmp.hist[tmp.hid];
        tmp.activeSpace = entry.space;
        tmp.cdir = entry.path;
        tmp.files = [];
      }
      break;
    }
    case 'CLOUD_FILES_LOADED': {
      tmp.files = action.payload;
      tmp.loading = false;
      tmp.error = null;
      break;
    }
    case 'CLOUD_LOADING': {
      tmp.loading = true;
      break;
    }
    case 'CLOUD_ERROR': {
      tmp.error = action.payload;
      tmp.loading = false;
      break;
    }
    case 'CLOUD_VIEW': {
      tmp.view = action.payload; // 'explorer' | 'settings'
      break;
    }
    default:
      return state;
  }

  return tmp;
};

export default cloudReducer;
