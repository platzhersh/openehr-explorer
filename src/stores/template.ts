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

  async function fetchTemplates(serverId: string) {
    loading.value = true;
    error.value = null;
    try {
      templates.value = await invoke<TemplateSummary[]>("list_templates", { serverId });
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  async function fetchWebTemplate(serverId: string, templateId: string) {
    const requestId = ++webTemplateRequestId;
    loading.value = true;
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
      if (requestId === webTemplateRequestId) {
        loading.value = false;
      }
    }
  }

  async function fetchOpt(serverId: string, templateId: string) {
    const requestId = ++optRequestId;
    loading.value = true;
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
      if (requestId === optRequestId) {
        loading.value = false;
      }
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
