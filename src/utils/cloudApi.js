// OpenCloud API client — calls Rust backend via Tauri Commands

let invoke = null;

async function getInvoke() {
  if (invoke) return invoke;
  try {
    const tauri = await import('@tauri-apps/api/core');
    invoke = tauri.invoke;
  } catch {
    invoke = null;
  }
  return invoke;
}

// ── Cloud Management (persisted in Rust) ──

export async function loadClouds() {
  const inv = await getInvoke();
  if (inv) return inv('cloud_load');
  // Browser fallback
  try { return JSON.parse(localStorage.getItem('kosmos-clouds') || '[]'); } catch { return []; }
}

export async function addCloud(name, url, token) {
  const inv = await getInvoke();
  if (inv) return inv('cloud_add', { name, url, token: token || null });
  const clouds = await loadClouds();
  clouds.push({ name, url, token });
  localStorage.setItem('kosmos-clouds', JSON.stringify(clouds));
  return clouds;
}

export async function removeCloud(index) {
  const inv = await getInvoke();
  if (inv) return inv('cloud_remove', { index });
  const clouds = await loadClouds();
  clouds.splice(index, 1);
  localStorage.setItem('kosmos-clouds', JSON.stringify(clouds));
  return clouds;
}

export async function updateToken(index, token) {
  const inv = await getInvoke();
  if (inv) return inv('cloud_update_token', { index, token });
  const clouds = await loadClouds();
  if (clouds[index]) clouds[index].token = token;
  localStorage.setItem('kosmos-clouds', JSON.stringify(clouds));
  return clouds;
}

// ── API (via Rust backend) ──

export async function getUser(url, token) {
  const inv = await getInvoke();
  if (inv) {
    const result = await inv('cloud_get_user', { url, token });
    return result.name;
  }
  const resp = await fetch(url.replace(/\/$/, '') + '/graph/v1.0/me', {
    headers: { 'Authorization': 'Bearer ' + token, 'Accept': 'application/json' },
  });
  const data = await resp.json();
  return data.displayName || data.mail || 'Unbekannt';
}

export async function listSpaces(url, token) {
  const inv = await getInvoke();
  if (inv) return inv('cloud_list_spaces', { url, token });
  const resp = await fetch(url.replace(/\/$/, '') + '/graph/v1.0/me/drives', {
    headers: { 'Authorization': 'Bearer ' + token, 'Accept': 'application/json' },
  });
  const data = await resp.json();
  return (data.value || []).map(d => ({ id: d.id, name: d.name, driveType: d.driveType }));
}

export async function listFiles(url, token, spaceId, path) {
  const inv = await getInvoke();
  if (inv) return inv('cloud_list_files', { url, token, spaceId, path: path || '/' });
  let endpoint;
  if (!path || path === '/') {
    endpoint = `/graph/v1.0/drives/${spaceId}/items/root/children`;
  } else {
    endpoint = `/graph/v1.0/drives/${spaceId}/items/root:/${path.replace(/^\//, '')}:/children`;
  }
  const resp = await fetch(url.replace(/\/$/, '') + endpoint, {
    headers: { 'Authorization': 'Bearer ' + token, 'Accept': 'application/json' },
  });
  const data = await resp.json();
  return (data.value || []).map(v => ({
    id: v.id, name: v.name, size: v.size || 0,
    mimeType: v.file?.mimeType || '', isFolder: !!v.folder,
    lastModified: v.lastModifiedDateTime || '',
  }));
}
