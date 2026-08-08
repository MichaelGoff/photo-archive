<script setup lang="ts">
import useTauriStore from '../composables/useTauriStore.ts';
import { SOURCE_PATH } from '../lib/storeKeys.ts';
import { ref, watch } from 'vue';
import { DirEntry, readDir } from '@tauri-apps/plugin-fs';
import { useErrorStore } from '../stores/useErrorStore.ts';
import { join } from '@tauri-apps/api/path';

const sourcePath = useTauriStore(SOURCE_PATH, '');
const directories = ref<DirEntry[]>([]);
const errorStore = useErrorStore();

async function loadFiles(path: string): Promise<DirEntry[]> {
  if (sourcePath.value) {
    try {
      const entries = await readDir(path);
      errorStore.clearError();
      return entries.filter((entry) => entry.isDirectory);
    } catch (error) {
      errorStore.setError(error);
      return [];
    }
  }
  return [];
}

watch(
  sourcePath,
  async () => {
    if (sourcePath.value) {
      directories.value = await loadFiles(sourcePath.value);
    }
  },
  { immediate: true },
);

async function mapDir(dir: DirEntry, parentDirPath: string): Promise<TreeItem> {
  return {
    id: await join(parentDirPath, dir.name),
    title: dir.name,
    children: [],
  };
}

watch(directories, async (val) => {
  items.value = await Promise.all(val.map((dir) => mapDir(dir, sourcePath.value)));
});

type TreeItem = {
  // Path of the tree item
  id: string;
  // Folder name
  title: string;
  // Child folders
  children?: TreeItem[];
};

const items = ref<TreeItem[]>([]);

async function loadChildren(item: unknown): Promise<void> {
  // I know it is a tree item. It's coming in from my items list.
  const treeItem = item as TreeItem;
  const childDirs = await loadFiles(treeItem.id);
  const childTreeItems = await Promise.all(childDirs.map((dir) => mapDir(dir, treeItem.id)));
  treeItem.children = childTreeItems.length ? childTreeItems : undefined;
}
</script>

<template>
  <v-treeview
    :items="items"
    :load-children="loadChildren"
  >
  </v-treeview>
</template>

<style scoped></style>
