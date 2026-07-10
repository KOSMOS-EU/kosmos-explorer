import React from 'react';
import './index.css';
import './short.css';

import {Explorer} from './containers/applications/apps/explorer';
import ActMenu from './components/menu';

function App() {
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
