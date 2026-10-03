import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';
import { ArrowLeft, ArrowUpRight, File, FolderOpen, Search, X } from 'lucide-react';
import './style.css';

type FileRecord = { id:number; name:string; original_name:string; path:string; original_path:string; size_bytes:number; first_seen_at:string; last_seen_at:string; is_present:boolean; source_url:string|null; source_page_url:string|null; referrer_url:string|null; source_domain:string|null; browser_name:string|null; browser_profile:string|null; source_confidence:string|null };
type FileEvent = { event_type:string; at:string; old_path:string|null; new_path:string|null };
type FileFacts = { created_at:number|null; modified_at:number|null; file_type:string; zone:string|null };

const date = (value:string) => new Date(value.replace(' ', 'T') + 'Z').toLocaleString();
const diskDate = (value:number|null) => value == null ? 'Unavailable' : new Date(value * 1000).toLocaleString();
const size = (value:number) => value < 1024 ? `${value} B` : value < 1048576 ? `${(value/1024).toFixed(1)} KB` : value < 1073741824 ? `${(value/1048576).toFixed(1)} MB` : `${(value/1073741824).toFixed(1)} GB`;
const title = (value:string) => value.replaceAll('_', ' ').toLowerCase().replace(/^./, letter => letter.toUpperCase());

export default function App() {
  const [files, setFiles] = useState<FileRecord[]>([]);
  const [query, setQuery] = useState('');
  const [selected, setSelected] = useState<FileRecord|null>(null);
  const [events, setEvents] = useState<FileEvent[]>([]);
  const [facts, setFacts] = useState<FileFacts|null>(null);
  const [error, setError] = useState('');

  const inspectPath = useCallback((path:string) => {
    invoke<FileRecord|null>('inspect_file', {path})
      .then(file => file ? setSelected(file) : setError('No record found for this file.'))
      .catch(reason => setError(String(reason)));
  }, []);

  useEffect(() => {
    let active = true;
    invoke<string|null>('take_pending_path').then(path => { if (active && path) inspectPath(path); }).catch(reason => setError(String(reason)));
    const unlisten = listen<string>('inspect-path', event => inspectPath(event.payload));
    return () => { active = false; unlisten.then(stop => stop()); };
  }, [inspectPath]);

  useEffect(() => {
    invoke<FileRecord[]>('list_files', {query}).then(setFiles).catch(reason => setError(String(reason)));
    const unlisten = listen('history-changed', () => invoke<FileRecord[]>('list_files', {query}).then(setFiles).catch(() => {}));
    return () => { unlisten.then(stop => stop()); };
  }, [query]);

  useEffect(() => {
    const window = getCurrentWindow();
    window.setSize(new LogicalSize(selected ? 620 : 760, 720)).catch(() => {});
    if (!selected) return;
    invoke<FileEvent[]>('file_events', {fileId:selected.id}).then(setEvents).catch(reason => setError(String(reason)));
    invoke<FileFacts>('file_facts', {path:selected.path}).then(setFacts).catch(reason => setError(String(reason)));
  }, [selected]);

  const openExplorer = () => selected && invoke('show_in_explorer', {path:selected.path}).catch(reason => setError(String(reason)));
  const modelId = selected?.source_page_url?.match(/makerworld\.com\/[^/]+\/models\/(\d+)/i)?.[1];
  const openAddress = (address:string) => invoke('open_source_url', {address}).catch(reason => setError(String(reason)));

  return <div className="app">
    <header className="topbar"><div className="brand"><span className="brand-mark">● · · ›</span><strong>Breadcrumb</strong></div><span className="topbar-label">File history</span></header>
    <main className={selected ? 'inspector' : 'browser'}>
      {selected ? <>
        <div className="toolbar"><button className="text-button" onClick={() => {setSelected(null); setFacts(null);}}><ArrowLeft size={16}/> All files</button><button className="text-button" onClick={openExplorer}><FolderOpen size={16}/> Show in Explorer</button></div>
        <div className="identity"><div className="identity-icon"><File size={27}/></div><div className="identity-text"><h1 title={selected.name}>{selected.name}</h1><p title={selected.path}>{selected.path}</p></div></div>
        <div className="summary"><span className="summary-label">File trail</span><strong>{selected.source_domain ? `From ${selected.source_domain}` : facts?.zone === 'Internet' ? 'From the internet' : 'Origin unknown'}</strong><span>{selected.source_page_url ? 'Download page matched from browser history.' : selected.source_url ? 'A source address was saved for this file.' : facts?.zone === 'Internet' ? 'Windows marked this file as downloaded; its address was not saved.' : 'Breadcrumb first saw this file at the location below.'}</span></div>
        <section className="panel"><h2>Origin</h2><dl>
          {modelId && <><dt>Model ID</dt><dd>{modelId}</dd></>}
          {selected.source_page_url && <><dt>Download page</dt><dd className="path-value"><button className="address-button" onClick={() => openAddress(selected.source_page_url!)}>{selected.source_page_url} <ArrowUpRight size={13}/></button></dd></>}
          <dt>First seen</dt><dd>{date(selected.first_seen_at)}</dd>
          <dt>Original name</dt><dd>{selected.original_name}</dd>
          <dt>Original location</dt><dd className="path-value">{selected.original_path}</dd>
          {selected.source_url && <><dt>File URL</dt><dd className="path-value">{selected.source_url}</dd></>}
          {selected.referrer_url && <><dt>Referrer</dt><dd className="path-value">{selected.referrer_url}</dd></>}
          {selected.browser_name && <><dt>Browser</dt><dd>{selected.browser_name}{selected.browser_profile ? ` · ${selected.browser_profile}` : ''}</dd></>}
          {facts?.zone && <><dt>Windows zone</dt><dd>{facts.zone}</dd></>}
        </dl></section>
        <section className="panel"><h2>File details</h2><dl>
          <dt>Current location</dt><dd className="path-value">{selected.path}</dd>
          <dt>Type</dt><dd>{facts?.file_type ?? 'File'}</dd>
          <dt>Size</dt><dd>{size(selected.size_bytes)}</dd>
          <dt>Created on disk</dt><dd>{diskDate(facts?.created_at ?? null)}</dd>
          <dt>Last modified</dt><dd>{diskDate(facts?.modified_at ?? null)}</dd>
          <dt>Status</dt><dd>{selected.is_present ? 'Present' : 'No longer at this location'}</dd>
        </dl></section>
        <section className="panel history-panel"><h2>History <span>{events.length} events</span></h2>{events.length ? <ol className="timeline">{events.map((event, index) => <li key={`${event.at}-${index}`}><span className="timeline-dot"/><div><strong>{title(event.event_type)}</strong><time>{date(event.at)}</time>{event.old_path && <small>From {event.old_path}</small>}{event.new_path && event.new_path !== selected.path && <small>To {event.new_path}</small>}</div></li>)}</ol> : <p className="muted">No recorded changes yet.</p>}</section>
      </> : <>
        <div className="browser-heading"><h1>File history</h1><p>Find a file and see where it came from.</p></div>
        <label className="search"><Search size={18}/><input autoFocus placeholder="Search name, path, or website" value={query} onChange={event => setQuery(event.target.value)}/>{query && <button aria-label="Clear search" onClick={() => setQuery('')}><X size={17}/></button>}</label>
        <div className="list-heading">{query ? 'Search results' : 'Recent files'} <span>{files.length}</span></div>
        <div className="file-list">{files.length ? files.map(file => <button className="file-row" key={file.id} onClick={() => setSelected(file)}><span className="row-icon"><File size={20}/></span><span className="row-text"><strong>{file.name}</strong><small>{file.source_domain || file.path}</small></span><span className="row-date">{date(file.first_seen_at)}</span><ArrowUpRight size={16}/></button>) : <div className="empty"><FolderOpen size={30}/><strong>{query ? 'No matching files' : 'No files recorded yet'}</strong><span>{query ? 'Try a filename or another search term.' : 'New files in watched folders will appear here.'}</span></div>}</div>
      </>}
    </main>
    {error && <div className="error" role="alert" onClick={() => setError('')}>{error}<X size={15}/></div>}
  </div>;
}
