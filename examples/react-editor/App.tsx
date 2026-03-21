/**
 * Example: Embed Rudra Office editor in a React application.
 *
 * Usage:
 *   npm install @rudra/react @rudra/editor @rudra/wasm
 *   npm run dev
 */

import React, { useRef, useState } from 'react';
import { S1EditorComponent, type S1EditorRef } from '@rudra/react';

function App() {
  const editorRef = useRef<S1EditorRef>(null);
  const [status, setStatus] = useState('Ready');

  const handleReady = () => {
    setStatus('Editor loaded');
  };

  const handleChange = () => {
    setStatus('Document modified');
  };

  const handleOpen = async () => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.docx,.odt,.txt,.md';
    input.onchange = async (e) => {
      const file = (e.target as HTMLInputElement).files?.[0];
      if (!file) return;
      const buffer = await file.arrayBuffer();
      editorRef.current?.open(buffer);
      setStatus(`Opened: ${file.name}`);
    };
    input.click();
  };

  const handleExportPdf = async () => {
    try {
      const blob = await editorRef.current?.exportDocument('pdf');
      if (blob) {
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = 'document.pdf';
        a.click();
        URL.revokeObjectURL(url);
        setStatus('PDF exported');
      }
    } catch (e) {
      setStatus(`Export failed: ${e}`);
    }
  };

  return (
    <div style={{ height: '100vh', display: 'flex', flexDirection: 'column' }}>
      <header style={{ padding: '8px 16px', borderBottom: '1px solid #e0e0e0', display: 'flex', gap: '8px', alignItems: 'center' }}>
        <h3 style={{ margin: 0, fontSize: '16px' }}>s1engine React Example</h3>
        <button onClick={handleOpen}>Open File</button>
        <button onClick={() => editorRef.current?.createNew()}>New</button>
        <button onClick={handleExportPdf}>Export PDF</button>
        <span style={{ marginLeft: 'auto', fontSize: '12px', color: '#666' }}>{status}</span>
      </header>
      <S1EditorComponent
        ref={editorRef}
        theme="default"
        toolbar="standard"
        onReady={handleReady}
        onChange={handleChange}
        style={{ flex: 1 }}
      />
    </div>
  );
}

export default App;
