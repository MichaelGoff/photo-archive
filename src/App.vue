<script setup lang="ts">
//import { invoke } from '@tauri-apps/api/core';
import SettingsAction from './components/SettingsAction.vue';

// async function greet() {
//   // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
//   greetMsg.value = await invoke('greet', { name: name.value });
// }
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
      <v-card
        class="folder-tree mx-4 mt-4 flex-grow-1"
        variant="tonal"
      >
        <v-card-text class="text-medium-emphasis"> No source folder... </v-card-text>
      </v-card>
      <div class="d-flex ma-4">
        <v-spacer />
        <v-btn text="Archive"></v-btn>
      </div>
    </v-main>
  </v-app>
</template>

<style scoped></style>
