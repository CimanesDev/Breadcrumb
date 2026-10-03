import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { ArrowLeft, ChevronRight, File, FolderOpen, Search, X } from 'lucide-react';

type Record = {id:number;name:string;original_name:string;path:string;original_path:string;size_bytes:number;first_seen_at:string;last_seen_at:string;is_present:boolean;source_url:string|null;referrer_url:string|null;source_domain:string|null};
type Event = {event_type:string;at:string;old_path:string|null;new_path:string|null};
const date = (value:string) => new Date(value.replace(' ', 'T')+'Z').toLocaleString();
const size = (value:number) => value < 1024 ? `${value} B` : value < 1048576 ? `${(value/1024).toFixed(1)} KB` : `${(value/1048576).toFixed(1)} MB`;

export default function App() {
  const [files,setFiles] = useState<Record[]>([]);
  const [query,setQuery] = useState('');
  const [selected,setSelected] = useState<Record|null>(null);
  const [events,setEvents] = useState<Event[]>([]);
  const [folder,setFolder] = useState('Downloads');
  const [error,setError] = useState('');
  const refresh = () => invoke<Record[]>('list_files',{query}).then(setFiles).catch(e=>setError(String(e)));
  useEffect(()=>{ refresh(); },[query]);
  useEffect(()=>{ invoke<string>('watched_folder').then(setFolder).catch(()=>{}); const unlisten=listen('history-changed',refresh); return ()=>{unlisten.then(fn=>fn());}; },[query]);
  useEffect(()=>{ if(selected) invoke<Event[]>('file_events',{fileId:selected.id}).then(setEvents).catch(e=>setError(String(e))); },[selected]);
  return <div className="app">
    <header><div className="brand"><span className="brand-mark">● <i>•</i> <i>•</i> ›</span><strong>Breadcrumb</strong></div><span className="status"><span/> Watching Downloads</span></header>
    <main>
      {selected ? <><button className="back" onClick={()=>setSelected(null)}><ArrowLeft size={17}/> Back to files</button><div className="detail-head"><div className="file-icon"><File size={28}/></div><div><h1>{selected.name}</h1><p>{selected.path}</p></div></div>
        <div className="detail-grid"><section className="card origin"><label>ORIGIN</label><h2>{selected.source_domain || 'Source unknown'}</h2><p>{selected.source_url ? 'Downloaded from this address' : 'No source metadata was found for this file.'}</p>{selected.source_url && <div className="url">{selected.source_url}</div>}{selected.referrer_url && <p>Referrer: {selected.referrer_url}</p>}<small>{selected.source_url ? 'High confidence · Windows download metadata' : 'Breadcrumb will show a source only when evidence is available.'}</small></section>
        <section className="card"><label>FILE</label><dl><dt>Original name</dt><dd>{selected.original_name}</dd><dt>Original location</dt><dd>{selected.original_path}</dd><dt>Current location</dt><dd>{selected.path}</dd><dt>Size</dt><dd>{size(selected.size_bytes)}</dd><dt>First seen</dt><dd>{date(selected.first_seen_at)}</dd><dt>Status</dt><dd>{selected.is_present ? 'On this computer' : 'Deleted or moved outside watched folders'}</dd></dl></section></div>
        <section className="history"><label>HISTORY</label>{events.map((e,i)=><div className="history-row" key={i}><div className="history-dot"/><div><strong>{e.event_type.replace('_',' ').toLowerCase()}</strong><time>{date(e.at)}</time>{e.old_path && <p>From {e.old_path}</p>}{e.new_path && <p>To {e.new_path}</p>}</div></div>)}</section></>
      : <><div className="hero"><span className="eyebrow">YOUR FILE HISTORY</span><h1>Every file leaves a trail.</h1><p>Find where your files came from and what happened to them.</p></div><div className="search"><Search size={20}/><input placeholder="Search files, folders, or websites..." value={query} onChange={e=>setQuery(e.target.value)}/>{query && <button onClick={()=>setQuery('')} aria-label="Clear"><X size={17}/></button>}</div>
      <div className="section-title"><span>{query ? 'SEARCH RESULTS' : 'RECENT FILES'}</span><span>{files.length} files</span></div><div className="file-list">{files.length ? files.map(file=><button className="file-row" key={file.id} onClick={()=>setSelected(file)}><span className="file-icon"><File size={21}/></span><span className="file-main"><strong>{file.name}</strong><small>{file.source_domain || 'Source unknown'} · {date(file.first_seen_at)}</small></span><span className="file-size">{size(file.size_bytes)}</span><ChevronRight size={17}/></button>) : <div className="empty"><FolderOpen size={32}/><h3>{query?'No matching files':'Waiting for your first file'}</h3><p>{query?'Try a different filename or website.':`New files in ${folder} will appear here.`}</p></div>}</div></>}
    </main><footer><span>Your file history stays on your computer. Breadcrumb does not upload your files or browsing history.</span><span>v0.1</span></footer>{error && <div className="error" onClick={()=>setError('')}>{error} <X size={14}/></div>}
  </div>;
}
