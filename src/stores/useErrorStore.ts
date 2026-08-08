import { defineStore } from 'pinia';
import { ref } from 'vue';

export const useErrorStore = defineStore('error', () => {
  const errorMessage = ref('');

  function setError(error: unknown) {
    errorMessage.value = typeof error === 'string' ? error : (error as Error)?.message;
  }

  function clearError() {
    errorMessage.value = '';
  }

  return { errorMessage, setError, clearError };
});
