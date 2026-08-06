import { LazyStore } from '@tauri-apps/plugin-store';
import { nextTick, onScopeDispose, type Ref, ref, watch } from 'vue';

const store = new LazyStore('settings.json');

export default function useTauriStore<T>(key: string, defaultValue: T): Ref<T> {
  const state = ref<T>(defaultValue) as Ref<T>;
  let isUpdatingFromRust = false;
  let unlisten: (() => void) | null = null;

  store.get<T>(key).then((val) => {
    if (val !== null && val !== undefined) {
      state.value = val;
    }
  });

  store
    .onKeyChange(key, (newState: T | undefined) => {
      if (newState !== null && newState !== undefined) {
        // Prevent loopback notifications
        isUpdatingFromRust = true;
        state.value = newState;

        void nextTick(() => {
          isUpdatingFromRust = false;
        });
      }
    })
    .then((unlistenFn) => {
      unlisten = unlistenFn;
    });

  watch(
    state,
    async (newVal) => {
      if (isUpdatingFromRust) return;

      await store.set(key, newVal);
      await store.save();
    },
    { deep: true },
  );

  onScopeDispose(() => {
    if (unlisten) {
      unlisten();
    }
  });

  return state;
}
