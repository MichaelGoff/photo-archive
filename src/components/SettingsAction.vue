<script setup lang="ts">
import { mdiCog } from '@mdi/js';
import { ref } from 'vue';
import useTauriStore from '../composables/useTauriStore.ts';
import { SOURCE_PATH, DESTINATION_PATH } from '../lib/storeKeys.ts';
import { invoke } from '@tauri-apps/api/core';
import { useErrorStore } from '../stores/useErrorStore.ts';

const showDialog = ref(false);

const errorStore = useErrorStore();
const sourcePath = useTauriStore(SOURCE_PATH, '');
const destinationPath = useTauriStore(DESTINATION_PATH, '');

async function pickFolder(): Promise<string> {
  try {
    const folderPath = await invoke<string>('select_and_allow_directory');
    return folderPath ?? '';
  } catch (error) {
    errorStore.setError(error);
    return '';
  }
}

async function onSourceBrowse() {
  const folder = await pickFolder();
  if (folder) {
    sourcePath.value = folder;
  }
}

async function onDestinationBrowse() {
  const folder = await pickFolder();
  if (folder) {
    destinationPath.value = folder;
  }
}
</script>

<template>
  <v-icon-btn
    :icon="mdiCog"
    @click="showDialog = true"
  ></v-icon-btn>
  <v-dialog
    v-model="showDialog"
    width="90%"
    height="90%"
  >
    <v-form class="flex-grow-1">
      <v-card title="Settings">
        <v-card-text>
          Configure your source and destination folders.
          <v-text-field
            :model-value="sourcePath"
            readonly
            density="comfortable"
            label="Source Folder"
            class="mt-3"
          >
            <template #append>
              <v-btn
                text="Browse"
                variant="tonal"
                @click="onSourceBrowse"
              />
            </template>
          </v-text-field>
          <v-text-field
            :model-value="destinationPath"
            readonly
            density="comfortable"
            label="Destination Folder"
          >
            <template #append>
              <v-btn
                text="Browse"
                variant="tonal"
                @click="onDestinationBrowse"
              />
            </template>
          </v-text-field>
        </v-card-text>
        <v-card-actions>
          <v-spacer />
          <v-btn
            variant="text"
            text="Close"
            @click="showDialog = false"
          />
        </v-card-actions>
      </v-card>
    </v-form>
  </v-dialog>
</template>
