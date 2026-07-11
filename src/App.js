import React, {useEffect} from 'react';
import {useDispatch} from 'react-redux';
import './index.css';
import './short.css';

import {Explorer} from './containers/applications/apps/explorer';
import ActMenu from './components/menu';
import {loadClouds} from './utils/cloudApi';

function App() {
  const dispatch = useDispatch();

  // Load clouds from Rust backend on startup
  useEffect(() => {
    loadClouds().then(list => {
      dispatch({type: 'CLOUD_SET_LIST', payload: list});
    });
  }, []);

  // Context menu handler (from original Win11web)
  useEffect(() => {
    window.oncontextmenu = (e) => {
      e.preventDefault();
      var data = {
        top: e.clientY,
        left: e.clientX
      };

      // Walk up the DOM to find the element with data-menu
      var target = e.target;
      while (target && target !== document.body) {
        if (target.dataset && target.dataset.menu != null) {
          data.menu = target.dataset.menu;
          data.attr = target.attributes;
          data.dataset = target.dataset;
          dispatch({
            type: 'MENUSHOW',
            payload: data
          });
          return;
        }
        target = target.parentElement;
      }

      dispatch({type: 'MENUHIDE'});
    };

    window.onclick = () => {
      dispatch({type: 'MENUHIDE'});
    };
  }, []);

  return (
    <div className="App">
      <div className="appwrap" style={{background: 'var(--bg1)', height: '100vh'}}>
        <Explorer standalone={true}/>
        <ActMenu/>
      </div>
    </div>
  );
}

export default App;
