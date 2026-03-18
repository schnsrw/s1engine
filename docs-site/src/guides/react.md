# Embed in React

## Install

```bash
npm install @s1engine/react @s1engine/editor @s1engine/wasm
```

## Usage

```tsx
import { S1EditorComponent } from '@s1engine/react';
import { useRef } from 'react';

function MyEditor() {
  const editorRef = useRef(null);

  return (
    <div style={{ height: '100vh' }}>
      <S1EditorComponent
        ref={editorRef}
        theme="default"
        toolbar="standard"
        onReady={() => console.log('Editor ready')}
        onChange={(e) => console.log('Changed:', e.type)}
      />
    </div>
  );
}
```

## Open a File

```tsx
const handleOpen = async (file: File) => {
  const buffer = await file.arrayBuffer();
  editorRef.current?.open(buffer);
};
```

## Export

```tsx
const handleExport = async () => {
  const blob = await editorRef.current?.exportDocument('pdf');
  // Download or upload blob
};
```

## With Collaboration

```tsx
<S1EditorComponent
  collab={{
    serverUrl: 'ws://localhost:8787',
    roomId: 'my-document',
    userName: 'Alice',
  }}
/>
```
