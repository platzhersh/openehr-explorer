import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

export const useCompositionStore = defineStore("composition", () => {
  const loading = ref(false);
  const error = ref<string | null>(null);

  async function createComposition(
    serverId: string,
    ehrId: string,
    templateId: string,
    compositionData: Record<string, unknown>,
  ) {
    loading.value = true;
    error.value = null;
    try {
      const compositionUid = await invoke<string>("create_composition", {
        serverId,
        ehrId,
        templateId,
        compositionData,
      });
      return compositionUid;
    } catch (e) {
      error.value = String(e);
      throw e;
    } finally {
      loading.value = false;
    }
  }

  async function updateComposition(
    serverId: string,
    ehrId: string,
    compositionUid: string,
    compositionData: Record<string, unknown>,
  ) {
    loading.value = true;
    error.value = null;
    try {
      const newUid = await invoke<string>("update_composition", {
        serverId,
        ehrId,
        compositionUid,
        compositionData,
      });
      return newUid;
    } catch (e) {
      error.value = String(e);
      throw e;
    } finally {
      loading.value = false;
    }
  }

  async function deleteComposition(serverId: string, ehrId: string, compositionUid: string) {
    loading.value = true;
    error.value = null;
    try {
      await invoke<string>("delete_composition", {
        serverId,
        ehrId,
        compositionUid,
      });
    } catch (e) {
      error.value = String(e);
      throw e;
    } finally {
      loading.value = false;
    }
  }

  return {
    loading,
    error,
    createComposition,
    updateComposition,
    deleteComposition,
  };
});
