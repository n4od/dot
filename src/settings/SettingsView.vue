<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'

/**
 * Phase-1 settings: enough to prove the second window and the IPC round trip.
 * The real version reuses the customiser (shape, colour, expression) and writes
 * to the pet's config file.
 */

function quit() {
  void invoke('quit_app')
}
</script>

<template>
  <main>
    <h1>Dot</h1>
    <p class="lede">
      One transparent window per pet. Right now the pet reads the global cursor to turn its head,
      plays an emote every so often, and lets clicks through everywhere except on its body.
    </p>

    <dl>
      <dt>Pet window</dt>
      <dd>180 × 180, frameless, always on top, skipped from the taskbar</dd>
      <dt>Click-through</dt>
      <dd>On by default; switched off per frame when the cursor is over the silhouette</dd>
      <dt>Drag</dt>
      <dd>Press and hold the pet, then move</dd>
      <dt>Emote</dt>
      <dd>Double-click the pet</dd>
    </dl>

    <div class="actions">
      <button type="button" class="ghost" @click="quit">Quit Dot</button>
    </div>
  </main>
</template>

<style scoped>
main {
  padding: 28px 32px;
  max-width: 560px;
}

h1 {
  margin: 0 0 8px;
  font-size: 20px;
  font-weight: 600;
  letter-spacing: -0.01em;
}

.lede {
  margin: 0 0 24px;
  color: #55555f;
}

dl {
  display: grid;
  grid-template-columns: 130px 1fr;
  gap: 10px 16px;
  margin: 0 0 28px;
  padding: 18px 20px;
  border-radius: 12px;
  background: #f3f3f6;
}

dt {
  font-weight: 600;
  color: #2a2a33;
}

dd {
  margin: 0;
  color: #55555f;
}

.actions {
  display: flex;
  gap: 10px;
}

button {
  font: inherit;
  padding: 8px 14px;
  border-radius: 9px;
  cursor: pointer;
}

.ghost {
  border: 1px solid #d8d8e0;
  background: #fff;
  color: #2a2a33;
}

.ghost:hover {
  background: #f6f6f9;
}
</style>