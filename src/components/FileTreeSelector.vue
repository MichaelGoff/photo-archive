<script setup lang="ts">
import useTauriStore from '../composables/useTauriStore.ts';
import { SOURCE_PATH } from '../lib/storeKeys.ts';
import { ref, watch } from 'vue';
import { DirEntry, readDir } from '@tauri-apps/plugin-fs';
import { useErrorStore } from '../stores/useErrorStore.ts';
import { join } from '@tauri-apps/api/path';
import { mdiFolder, mdiFolderOpen, mdiImageMultiple, mdiFolderAlertOutline } from '@mdi/js';

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

// Helper function to determine the appropriate icon based on item state
function getItemIcon(item: TreeItem, isOpen: boolean): string {
  if (item.selectable) {
    return mdiImageMultiple;
  } else if (item.isValidStructure === false) {
    // Irregular directories get alert outline icon
    return mdiFolderAlertOutline;
  } else if (isOpen && item.children?.length) {
    return mdiFolderOpen;
  } else {
    return mdiFolder;
  }
}

// Checks if directory name follows YYYY_MM_DD structure
function isDateDirectory(name: string): boolean {
  return /^\d{4}_\d{2}_\d{2}$/.test(name);
}

// Checks if directory name follows YYYY_MM structure
function isMonthDirectory(name: string): boolean {
  return /^\d{4}_\d{2}$/.test(name);
}

// Checks if directory name follows YYYY structure
function isYearDirectory(name: string): boolean {
  return /^\d{4}$/.test(name);
}

async function countAllFilesInDirectory(path: string): Promise<number> {
  try {
    const entries = await readDir(path);
    return entries.length;
  } catch {
    return 0;
  }
}

async function mapDir(dir: DirEntry, parentDirPath: string): Promise<TreeItem> {
  const fullPath = await join(parentDirPath, dir.name);

  // Determine if this is a date directory (selectable)
  const isDateDir = isDateDirectory(dir.name);
  const isMonthDir = isMonthDirectory(dir.name);
  const isYearDir = isYearDirectory(dir.name);

  if (isDateDir) {
    // For leaf nodes, count all files
    const fileCount = await countAllFilesInDirectory(fullPath);
    return {
      id: fullPath,
      title: dir.name,
      selectable: true,
      fileCount: fileCount,
      isValidStructure: true,
    };
  } else if (isMonthDir || isYearDir) {
    // For parent directories, show as expandable but not selectable
    return {
      id: fullPath,
      title: dir.name,
      children: [],
      selectable: false,
      isValidStructure: true,
    };
  } else {
    // For directories with irregular structure
    return {
      id: fullPath,
      title: dir.name,
      children: [],
      selectable: false, // Never selectable, even if they don't have children
      isValidStructure: false,
    };
  }
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
  // Whether this item is selectable
  selectable?: boolean;
  // Number of files in this directory (for leaf nodes)
  fileCount?: number;
  // Whether this directory follows expected structure
  isValidStructure?: boolean;
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

  let childTreeItems: TreeItem[];

  if (isDateDirectory(treeItem.title)) {
    // For date directories, no children (leaf nodes)
    childTreeItems = [];
  } else {
    // For year and month directories, show all children including irregular ones
    childTreeItems = await Promise.all(childDirs.map((dir) => mapDir(dir, treeItem.id)));
  }

  treeItem.children = childTreeItems.length ? childTreeItems : undefined;
}

function itemProps(item: Record<string, unknown>) {
  // cast back to tree item that we know it is.
  const treeItem = item as TreeItem;
  return {
    selectable: treeItem.isValidStructure && treeItem.selectable,
    disabled: !treeItem.isValidStructure,
    prependIcon: getItemIcon(treeItem, openedItems.value.includes(treeItem.id)),
  };
}
</script>

<template>
  <v-treeview
    v-model:opened="openedItems"
    v-model:selected="activeIds"
    :items="items"
    item-value="id"
    :load-children="loadChildren"
    selectable
    density="compact"
    indent-lines
    :item-props="itemProps"
  >
    <template #append="{ item }">
      <div
        v-if="item.fileCount !== undefined"
        class="ml-2 text-disabled text-body-small"
      >
        ({{ item.fileCount }})
      </div>
    </template>
  </v-treeview>
</template>
