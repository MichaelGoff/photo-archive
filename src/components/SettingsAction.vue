<script setup lang="ts">
import { mdiCog } from '@mdi/js';
import { ref } from 'vue';
import { open } from '@tauri-apps/plugin-dialog';

const showDialog = ref(false);

const sourcePath = ref('');
const destinationPath = ref('');

async function pickFolder(): Promise<string> {
  const folderPath = await open({
    directory: true,
    multiple: false,
    title: 'Select folder',
  });

  return folderPath ?? '';
}

async function onSourceBrowse() {
  sourcePath.value = await pickFolder();
}

async function onDestinationBrowse() {
  destinationPath.value = await pickFolder();
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
            text="Cancel"
            @click="showDialog = false"
          />
          <v-btn
            color="primary"
            type="submit"
            text="Save"
          />
        </v-card-actions>
      </v-card>
    </v-form>
  </v-dialog>
</template>
