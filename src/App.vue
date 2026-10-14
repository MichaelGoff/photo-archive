<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import SettingsAction from './components/SettingsAction.vue';
import { SOURCE_PATH, DESTINATION_PATH } from './lib/storeKeys.ts';
import { DirEntry, readDir } from '@tauri-apps/plugin-fs';
import { computed, ref, watch } from 'vue';
import useTauriStore from './composables/useTauriStore.ts';
import { useErrorStore } from './stores/useErrorStore.ts';
import FileTreeSelector from './components/FileTreeSelector.vue';
import { listen } from '@tauri-apps/api/event';

// async function greet() {
//   // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
//   greetMsg.value = await invoke('greet', { name: name.value });
// }

const sourcePath = useTauriStore(SOURCE_PATH, '');
const destinationPath = useTauriStore(DESTINATION_PATH, '');
const directories = ref<DirEntry[]>([]);
const errorStore = useErrorStore();

const selectedPath = ref<string>('');
const archiveLoading = ref<boolean>(false);

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

type CopyProgress = {
  total_files: number;
  copied_files: number;
  current_file: string;
};

type ValidationReport = {
  total_files: number;
  verified_files: number;
  missing_files: string[];
  size_mismatches: string[];
  modified_time_mismatches: string[];
};

const totalFiles = ref<number>(0);
const copiedFiles = ref<number>(0);
const currentFile = ref<string>('');

const validationReport = ref<ValidationReport | null>(null);

const hasValidationIssues = computed(
  () =>
    validationReport.value !== null &&
    (validationReport.value.missing_files.length > 0 ||
      validationReport.value.size_mismatches.length > 0),
);

async function onArchive() {
  let unlisten: Awaited<ReturnType<typeof listen>> = () => {};
  try {
    archiveLoading.value = true;
    validationReport.value = null;
    unlisten = await listen<CopyProgress>('copy-progress', (event) => {
      totalFiles.value = event.payload.total_files;
      copiedFiles.value = event.payload.copied_files;
      currentFile.value = event.payload.current_file;
    });

    validationReport.value = await invoke<ValidationReport>('copy_folder_with_progress', {
      folderPathRaw: selectedPath.value,
    });
  } catch (error) {
    errorStore.setError(error);
  } finally {
    archiveLoading.value = false;
    unlisten();
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
      <div
        class="text-body-medium mx-4 mt-4"
        v-if="!destinationPath && selectedPath"
      >
        No destination folder configured...
      </div>
      <v-alert
        v-if="errorStore.errorMessage"
        type="error"
        :text="errorStore.errorMessage"
        variant="tonal"
        class="mx-4 mt-4"
        style="flex: 0 0 auto"
        border="start"
      />
      <v-alert
        v-if="validationReport"
        :type="hasValidationIssues ? 'error' : 'success'"
        variant="tonal"
        class="mx-4 mt-4"
        style="flex: 0 0 auto"
        border="start"
      >
        <div>
          {{
            hasValidationIssues
              ? `Validation failed: only ${validationReport.verified_files} of ${validationReport.total_files} files verified in the destination.`
              : `Archive complete: all ${validationReport.total_files} files verified in the destination.`
          }}
        </div>
        <div
          v-if="validationReport.missing_files.length"
          class="text-caption"
        >
          Missing: {{ validationReport.missing_files.join(', ') }}
        </div>
        <div
          v-if="validationReport.size_mismatches.length"
          class="text-caption"
        >
          Size mismatch: {{ validationReport.size_mismatches.join(', ') }}
        </div>
        <div
          v-if="validationReport.modified_time_mismatches.length"
          class="text-caption"
        >
          Modified time differs: {{ validationReport.modified_time_mismatches.join(', ') }}
        </div>
      </v-alert>
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
        <file-tree-selector
          v-else
          v-model="selectedPath"
        />
      </v-card>
      <div
        v-if="archiveLoading"
        class="mx-4 mt-2"
      >
        <div class="font-italic mb-1">Archiving {{ selectedPath }}</div>
        <v-progress-linear
          :model-value="copiedFiles"
          :max="totalFiles"
        />
        <span class="text-body-medium">{{ currentFile }}</span>
      </div>
      <div class="d-flex ma-4">
        <span
          v-if="selectedPath"
          class="border rounded py-1 px-2"
          >{{ selectedPath }}</span
        >
        <v-spacer />
        <v-btn
          text="Archive"
          color="primary"
          :disabled="!selectedPath || !destinationPath"
          :loading="archiveLoading"
          @click="onArchive"
        ></v-btn>
      </div>
    </v-main>
  </v-app>
</template>

<style scoped></style>
