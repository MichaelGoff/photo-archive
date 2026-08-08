<script setup lang="ts">
//import { invoke } from '@tauri-apps/api/core';
import SettingsAction from './components/SettingsAction.vue';
import { SOURCE_PATH } from './lib/storeKeys.ts';
import { DirEntry, readDir } from '@tauri-apps/plugin-fs';
import { ref, watch } from 'vue';
import useTauriStore from './composables/useTauriStore.ts';
import { useErrorStore } from './stores/useErrorStore.ts';
import FileTreeSelector from './components/FileTreeSelector.vue';

// async function greet() {
//   // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
//   greetMsg.value = await invoke('greet', { name: name.value });
// }

const sourcePath = useTauriStore(SOURCE_PATH, '');
const directories = ref<DirEntry[]>([]);
const errorStore = useErrorStore();

async function loadFiles() {
  if (sourcePath.value) {
    try {
      const entries = await readDir(sourcePath.value);
      directories.value = entries.filter((entry) => entry.isDirectory);
      errorStore.clearError();
    } catch (error) {
      errorStore.setError(error);
    }
  }
}

watch(
  sourcePath,
  () => {
    if (sourcePath.value) {
      loadFiles();
    }
  },
  { immediate: true },
);
loadFiles();
</script>

<template>
  <v-app>
    <v-app-bar
      title="Photo Archive"
      density="compact"
    >
      <template #append>
        <settings-action />
      </template>
    </v-app-bar>
    <v-main class="d-flex flex-column">
      <v-alert
        v-if="errorStore.errorMessage"
        type="error"
        :text="errorStore.errorMessage"
        variant="tonal"
        class="mx-4 mt-4"
        style="flex: 0 0 auto"
        border="start"
      />
      <v-card
        class="folder-tree mx-4 mt-4 flex-grow-1"
        variant="tonal"
      >
        <v-card-text
          v-if="!sourcePath"
          class="text-medium-emphasis"
        >
          No source folder...
        </v-card-text>
        <file-tree-selector v-else />
      </v-card>
      <div class="d-flex ma-4">
        <v-spacer />
        <v-btn text="Archive"></v-btn>
      </div>
    </v-main>
  </v-app>
</template>

<style scoped></style>
