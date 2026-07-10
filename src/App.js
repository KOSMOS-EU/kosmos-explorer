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
