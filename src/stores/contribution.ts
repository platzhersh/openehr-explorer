import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

export interface ContributionVersionRef {
  id: string;
  version_type: string | null;
}

export interface ContributionAudit {
  system_id: string | null;
  committer_name: string | null;
  time_committed: string | null;
  change_type: string | null;
  description: string | null;
}

export interface ContributionDetail {
  contribution_uid: string;
  audit: ContributionAudit | null;
  versions: ContributionVersionRef[];
}

export const useContributionStore = defineStore("contribution", () => {
  const detail = ref<ContributionDetail | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);

  async function fetchContribution(serverId: string, ehrId: string, contributionUid: string) {
    loading.value = true;
    error.value = null;
    detail.value = null;
    try {
      detail.value = await invoke<ContributionDetail>("get_contribution", {
        serverId,
        ehrId,
        contributionUid,
      });
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  return {
    detail,
    loading,
    error,
    fetchContribution,
  };
});
