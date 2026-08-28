import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

export interface TemplateSummary {
  template_id: string;
  concept: string | null;
  archetype_id: string | null;
  created_timestamp: string | null;
}

export const useTemplateStore = defineStore("template", () => {
  const templates = ref<TemplateSummary[]>([]);
  const loading = ref(false);
  const error = ref<string | null>(null);
  const selectedWebTemplate = ref<Record<string, unknown> | null>(null);
  const selectedOpt = ref<string | null>(null);

  // Selecting templates in quick succession fires overlapping requests per
  // template — without a guard, whichever response lands last "wins"
  // regardless of which template it was actually for, so a slower response
  // for a template the user has since navigated away from can silently
  // overwrite `selectedWebTemplate`/`selectedOpt` with stale content shown
  // (and, since the OPT download button uses the *current* template id for
  // the filename, downloadable) under the wrong template's name. Each fetch
  // stamps a per-kind counter and only applies its result if it's still the
  // most recent call of that kind.
  let webTemplateRequestId = 0;
  let optRequestId = 0;

  // fetchWebTemplate and fetchOpt run concurrently for a single template
  // selection. Each used to set the shared `loading` flag to false in its
  // own `finally`, so whichever of the two finished first flipped loading
  // off while the other was still in flight. Counting in-flight requests
  // instead means `loading` only goes false once every fetch has settled.
  let pendingRequests = 0;
  function beginLoading() {
    pendingRequests++;
    loading.value = true;
  }
  function endLoading() {
    pendingRequests = Math.max(0, pendingRequests - 1);
    loading.value = pendingRequests > 0;
  }

  async function fetchTemplates(serverId: string) {
    beginLoading();
    error.value = null;
    try {
      templates.value = await invoke<TemplateSummary[]>("list_templates", { serverId });
    } catch (e) {
      error.value = String(e);
    } finally {
      endLoading();
    }
  }

  async function fetchWebTemplate(serverId: string, templateId: string) {
    const requestId = ++webTemplateRequestId;
    beginLoading();
    error.value = null;
    try {
      const result = await invoke<Record<string, unknown>>("get_web_template", {
        serverId,
        templateId,
      });
      if (requestId === webTemplateRequestId) {
        selectedWebTemplate.value = result;
      }
    } catch (e) {
      if (requestId === webTemplateRequestId) {
        error.value = String(e);
      }
    } finally {
      endLoading();
    }
  }

  async function fetchOpt(serverId: string, templateId: string) {
    const requestId = ++optRequestId;
    beginLoading();
    try {
      const result = await invoke<string>("get_template_opt", {
        serverId,
        templateId,
      });
      if (requestId === optRequestId) {
        selectedOpt.value = result;
      }
    } catch (e) {
      if (requestId === optRequestId) {
        error.value = String(e);
      }
    } finally {
      endLoading();
    }
  }

  async function uploadTemplate(serverId: string, optXml: string) {
    return invoke<string>("upload_template", { serverId, optXml });
  }

  return {
    templates,
    loading,
    error,
    selectedWebTemplate,
    selectedOpt,
    fetchTemplates,
    fetchWebTemplate,
    fetchOpt,
    uploadTemplate,
  };
});
