import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

export interface EhrSummary {
  ehr_id: string;
  system_id: string | null;
  time_created: string | null;
  subject_id: string | null;
}

export interface CompositionSummary {
  uid: string;
  template_id: string | null;
  name: string | null;
  composer: string | null;
  time_committed: string | null;
}

export interface EhrDetail {
  ehr_id: string;
  system_id: string | null;
  time_created: string | null;
  is_modifiable: boolean | null;
  is_queryable: boolean | null;
  compositions: CompositionSummary[];
}

export interface EhrListResponse {
  ehrs: EhrSummary[];
  total: number;
  offset: number;
  limit: number;
}

export const useEhrStore = defineStore("ehr", () => {
  const ehrs = ref<EhrSummary[]>([]);
  const total = ref(0);
  const offset = ref(0);
  const limit = ref(20);
  const loading = ref(false);
  const error = ref<string | null>(null);
  const selectedEhr = ref<EhrDetail | null>(null);

  async function fetchEhrs(serverId: string, page = 0) {
    loading.value = true;
    error.value = null;
    try {
      const result = await invoke<EhrListResponse>("list_ehrs", {
        serverId,
        offset: page * limit.value,
        limit: limit.value,
      });
      ehrs.value = result.ehrs;
      total.value = result.total;
      offset.value = result.offset;
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  async function fetchEhrDetail(serverId: string, ehrId: string) {
    loading.value = true;
    error.value = null;
    try {
      selectedEhr.value = await invoke<EhrDetail>("get_ehr_detail", {
        serverId,
        ehrId,
      });
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  return {
    ehrs,
    total,
    offset,
    limit,
    loading,
    error,
    selectedEhr,
    fetchEhrs,
    fetchEhrDetail,
  };
});
