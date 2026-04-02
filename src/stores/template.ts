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
    loading.value = true;
    error.value = null;
    try {
      selectedWebTemplate.value = await invoke<Record<string, unknown>>(
        "get_web_template",
        { serverId, templateId }
      );
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  async function fetchOpt(serverId: string, templateId: string) {
    loading.value = true;
    try {
      selectedOpt.value = await invoke<string>("get_template_opt", {
        serverId,
        templateId,
      });
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
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
