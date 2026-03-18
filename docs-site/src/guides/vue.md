# Embed in Vue

## Install

```bash
npm install @s1engine/vue @s1engine/editor @s1engine/wasm
```

## Usage

```vue
<template>
  <S1EditorVue
    ref="editor"
    theme="default"
    toolbar="standard"
    @ready="onReady"
    @change="onChange"
    style="height: 100vh"
  />
</template>

<script setup>
import { ref } from 'vue';
import { S1EditorVue } from '@s1engine/vue';

const editor = ref(null);
const onReady = () => console.log('Ready');
const onChange = (e) => console.log('Changed:', e.type);

const openFile = async (file) => {
  const buffer = await file.arrayBuffer();
  editor.value?.open(buffer);
};
</script>
```
