<script setup lang="ts">
import useTauriStore from '../composables/useTauriStore.ts';
import { SOURCE_PATH } from '../lib/storeKeys.ts';
import { ref, watch } from 'vue';
import { DirEntry, readDir } from '@tauri-apps/plugin-fs';
import { useErrorStore } from '../stores/useErrorStore.ts';
import { join } from '@tauri-apps/api/path';
import { mdiFolder, mdiFolderOpen } from '@mdi/js';

const activePath = defineModel({
  type: String,
  required: true,
});

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

const openedItems = ref<string[]>([]);
const activeIds = ref<string[]>([]);
const items = ref<TreeItem[]>([]);

watch(activeIds, (val) => {
  if (val.length) {
    activePath.value = val[0];
  } else {
    activePath.value = '';
  }
});

watch(activePath, (val) => {
  if (val && activeIds.value?.[0] !== val) {
    activeIds.value = [val];
  }
});

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
    v-model:opened="openedItems"
    v-model:activated="activeIds"
    :items="items"
    item-value="id"
    :load-children="loadChildren"
    activatable
    density="compact"
  >
    <template #prepend="{ item, isOpen }">
      <v-icon :icon="isOpen && item.children?.length ? mdiFolderOpen : mdiFolder" />
    </template>
  </v-treeview>
</template>

<style scoped></style>
