// OpenCloud API client — calls Rust backend via Tauri Commands

let invoke = null;

async function getInvoke() {
  if (invoke) return invoke;
  try {
    const tauri = await import('@tauri-apps/api/core');
    invoke = tauri.invoke;
    return invoke;
  } catch {
    // Fallback for browser dev: direct fetch
    return null;
  }
}

export async function getUser(baseUrl, token) {
  const inv = await getInvoke();
  if (inv) {
    const result = await inv('cloud_get_user', { url: baseUrl, token });
    return result.name;
  }
  // Browser fallback
  const resp = await fetch(baseUrl.replace(/\/$/, '') + '/graph/v1.0/me', {
    headers: { 'Authorization': 'Bearer ' + token, 'Accept': 'application/json' },
  });
  const data = await resp.json();
  return data.displayName || data.mail || 'Unbekannt';
}

export async function listSpaces(baseUrl, token) {
  const inv = await getInvoke();
  if (inv) {
    return inv('cloud_list_spaces', { url: baseUrl, token });
  }
  const resp = await fetch(baseUrl.replace(/\/$/, '') + '/graph/v1.0/me/drives', {
    headers: { 'Authorization': 'Bearer ' + token, 'Accept': 'application/json' },
  });
  const data = await resp.json();
  return (data.value || []).map(d => ({ id: d.id, name: d.name, driveType: d.driveType }));
}

export async function listFiles(baseUrl, token, spaceId, path) {
  const inv = await getInvoke();
  if (inv) {
    return inv('cloud_list_files', { url: baseUrl, token, spaceId, path: path || '/' });
  }
  let endpoint;
  if (!path || path === '/') {
    endpoint = `/graph/v1.0/drives/${spaceId}/items/root/children`;
  } else {
    endpoint = `/graph/v1.0/drives/${spaceId}/items/root:/${path.replace(/^\//, '')}:/children`;
  }
  const resp = await fetch(baseUrl.replace(/\/$/, '') + endpoint, {
    headers: { 'Authorization': 'Bearer ' + token, 'Accept': 'application/json' },
  });
  const data = await resp.json();
  return (data.value || []).map(v => ({
    id: v.id, name: v.name, size: v.size || 0,
    mimeType: v.file?.mimeType || '', isFolder: !!v.folder,
    lastModified: v.lastModifiedDateTime || '',
  }));
}
