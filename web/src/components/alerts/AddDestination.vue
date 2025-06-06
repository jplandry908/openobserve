<!-- Copyright 2023 OpenObserve Inc.

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License
along with this program.  If not, see <http://www.gnu.org/licenses/>.
-->

<template>
  <q-page class="q-pa-none" style="min-height: inherit">
    <div class="o2-input">
      <div class="row items-center no-wrap q-mx-md q-my-sm">
        <div class="flex items-center">
          <div
            class="flex justify-center items-center q-mr-md cursor-pointer"
            style="
              border: 1.5px solid;
              border-radius: 50%;
              width: 22px;
              height: 22px;
            "
            title="Go Back"
            @click="$emit('cancel:hideform')"
          >
            <q-icon name="arrow_back_ios_new" size="14px" />
          </div>
          <div class="col" data-test="add-destination-title">
            <div v-if="destination" class="text-h6">
              {{ t("alert_destinations.updateTitle") }}
            </div>
            <div v-else class="text-h6">
              {{ t("alert_destinations.addTitle") }}
            </div>
          </div>
        </div>
      </div>
      <q-separator />
      <div class="row q-col-gutter-sm q-px-lg q-my-md">
        <div v-if="isAlerts" class="col-12 q-pb-md">
          <app-tabs
            style="
              border: 1px solid #8a8a8a;
              border-radius: 4px;
              overflow: hidden;
              width: fit-content;
            "
            :tabs="tabs"
            v-model:active-tab="formData.type"
          />
        </div>
        <div
          v-if="formData.type === 'email' && !getFormattedTemplates.length"
          class="flex items-center col-12 q-mb-md"
        >
          <div class="text-subtitle2 q-mr-sm">
            It looks like you haven't created any Email Templates yet.
          </div>
          <q-btn
            label="Create Email Template"
            size="sm"
            no-caps
            color="secondary"
            style="border-radius: 4px; font-size: 12px"
            @click="createEmailTemplate"
          />
        </div>
        <div
          class="q-py-xs"
          :class="{ 'col-6': isAlerts, 'col-12': !isAlerts }"
        >
          <q-input
            data-test="add-destination-name-input"
            v-model="formData.name"
            :label="t('alerts.name') + ' *'"
            color="input-border"
            bg-color="input-bg"
            class="showLabelOnTop"
            stack-label
            outlined
            filled
            dense
            v-bind:readonly="isUpdatingDestination"
            v-bind:disable="isUpdatingDestination"
            :rules="[
              (val: any) =>
                !!val
                  ? isValidResourceName(val) ||
                    `Characters like :, ?, /, #, and spaces are not allowed.`
                  : t('common.nameRequired'),
            ]"
            tabindex="0"
          />
        </div>
        <div v-if="isAlerts" class="col-6 row q-py-xs">
          <div class="col-12">
            <q-select
              data-test="add-destination-template-select"
              v-model="formData.template"
              :label="t('alert_destinations.template') + ' *'"
              :options="getFormattedTemplates"
              color="input-border"
              bg-color="input-bg"
              class="showLabelOnTop no-case"
              stack-label
              outlined
              filled
              dense
              :rules="[(val: any) => !!val || 'Field is required!']"
              tabindex="0"
            />
          </div>
        </div>

        <template
          v-if="(isAlerts && formData.type === 'http') || isAlerts == false"
        >
          <div class="col-6 q-py-xs">
            <q-input
              data-test="add-destination-url-input"
              v-model="formData.url"
              :label="t('alert_destinations.url') + ' *'"
              color="input-border"
              bg-color="input-bg"
              class="showLabelOnTop"
              stack-label
              outlined
              filled
              dense
              :rules="[(val: any) => !!val.trim() || 'Field is required!']"
              tabindex="0"
            />
          </div>
          <div  class=" q-py-xs destination-method-select"
          :class="{ 'col-3': !isAlerts, 'col-6': isAlerts }"
          >
            <q-select
              data-test="add-destination-method-select"
              v-model="formData.method"
              :label="t('alert_destinations.method') + ' *'"
              :options="apiMethods"
              color="input-border"
              bg-color="input-bg"
              class="showLabelOnTop"
              stack-label
              outlined
              :popup-content-style="{ textTransform: 'uppercase' }"
              filled
              dense
              :rules="[(val: any) => !!val || 'Field is required!']"
              tabindex="0"
            />
            
          </div>
          <div v-if="!isAlerts" class="col-3 q-py-xs destination-method-select">
            <q-select
              data-test="add-destination-output-format-select"
              v-model="formData.output_format"
              :label="t('alert_destinations.output_format') + ' *'"
              :options="outputFormats"
              color="input-border"
              bg-color="input-bg"
              class="showLabelOnTop"
              stack-label
              outlined
              :popup-content-style="{ textTransform: 'uppercase' }"
              filled
              dense
              :rules="[(val: any) => !!val || 'Field is required!']"
              tabindex="0"
            />
            
          </div>
          <div class="col-12 q-py-sm">
            <div class="text-bold q-py-xs" style="paddingleft: 10px">
              Headers
            </div>
            <div
              v-for="(header, index) in apiHeaders"
              :key="header.uuid"
              class="row q-col-gutter-sm q-pb-sm"
            >
              <div class="col-5 q-ml-none">
                <q-input
                  :data-test="`add-destination-header-${header['key']}-key-input`"
                  v-model="header.key"
                  color="input-border"
                  bg-color="input-bg"
                  stack-label
                  outlined
                  filled
                  :placeholder="t('alert_destinations.api_header')"
                  dense
                  tabindex="0"
                />
              </div>
              <div class="col-5 q-ml-none">
                <q-input
                  :data-test="`add-destination-header-${header['key']}-value-input`"
                  v-model="header.value"
                  :placeholder="t('alert_destinations.api_header_value')"
                  color="input-border"
                  bg-color="input-bg"
                  stack-label
                  outlined
                  filled
                  dense
                  isUpdatingDestination
                  tabindex="0"
                />
              </div>
              <div class="col-2 q-ml-none">
                <q-btn
                  :data-test="`add-destination-header-${header['key']}-delete-btn`"
                  icon="delete"
                  class="q-ml-xs iconHoverBtn"
                  padding="sm"
                  unelevated
                  size="sm"
                  round
                  flat
                  :title="t('alert_templates.edit')"
                  @click="deleteApiHeader(header)"
                />
                <q-btn
                  data-test="add-destination-add-header-btn"
                  v-if="index === apiHeaders.length - 1"
                  icon="add"
                  class="q-ml-xs iconHoverBtn"
                  padding="sm"
                  unelevated
                  size="sm"
                  round
                  flat
                  :title="t('alert_templates.edit')"
                  @click="addApiHeader()"
                />
              </div>
            </div>
          </div>
          <div class="col-12 q-py-sm">
            <div class="q-py-sm">
              <q-toggle
                data-test="add-destination-skip-tls-verify-toggle"
                class="q-mt-sm"
                v-model="formData.skip_tls_verify"
                :label="t('alert_destinations.skip_tls_verify')"
              />
            </div>
          </div>
        </template>
        <template v-if="formData.type === 'email'">
          <q-input
            v-model="formData.emails"
            :label="t('reports.recipients') + ' *'"
            color="input-border"
            bg-color="input-bg"
            class="showLabelOnTop"
            stack-label
            outlined
            filled
            dense
            :rules="[
              (val: any) =>
                /^([a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,})(\s*[;,]\s*([a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}))*$/.test(
                  val,
                ) || 'Add valid emails!',
            ]"
            tabindex="0"
            style="width: 100%"
            borderless
            :placeholder="t('user.inviteByEmail')"
          />
        </template>

        <template v-if="formData.type === 'action'">
          <div class="col-6 q-py-xs action-select">
            <q-select
              data-test="add-destination-action-select"
              v-model="formData.action_id"
              :label="t('alert_destinations.action') + ' *'"
              :options="filteredActions"
              color="input-border"
              bg-color="input-bg"
              class="showLabelOnTop no-case"
              map-options
              emit-value
              stack-label
              outlined
              filled
              dense
              use-input
              :loading="isLoadingActions"
              :rules="[(val: any) => !!val || 'Field is required!']"
              tabindex="0"
              @filter="filterActions"
            />
          </div>
        </template>
      </div>
    </div>
    <div class="flex justify-center q-mt-lg">
      <q-btn
        data-test="add-destination-cancel-btn"
        v-close-popup="true"
        class="q-mb-md text-bold"
        :label="t('alerts.cancel')"
        text-color="light-text"
        padding="sm md"
        no-caps
        @click="$emit('cancel:hideform')"
      />
      <q-btn
        data-test="add-destination-submit-btn"
        :label="t('alerts.save')"
        class="q-mb-md text-bold no-border q-ml-md"
        color="secondary"
        padding="sm xl"
        @click="saveDestination"
        no-caps
      />
    </div>
  </q-page>
</template>
<script lang="ts" setup>
import {
  ref,
  computed,
  defineProps,
  onBeforeMount,
  onActivated,
  defineEmits,
  reactive,
} from "vue";
import type { Ref, PropType } from "vue";
import { useI18n } from "vue-i18n";
import destinationService from "@/services/alert_destination";
import { useStore } from "vuex";
import { useQuasar } from "quasar";
import type {
  Template,
  DestinationData,
  Headers,
  DestinationPayload,
} from "@/ts/interfaces";
import { useRouter } from "vue-router";
import { isValidResourceName } from "@/utils/zincutils";
import AppTabs from "@/components/common/AppTabs.vue";
import actionService from "@/services/action_scripts";
import config from "@/aws-exports";
import useActions from "@/composables/useActions";

const props = defineProps({
  templates: {
    type: Array as PropType<Template[]>,
    default: [],
  },
  destination: {
    type: Object as PropType<DestinationPayload | null>,
    default: null,
  },
  isAlerts: {
    type: Boolean,
    default: true,
  },
});
const emit = defineEmits(["get:destinations", "cancel:hideform"]);
const q = useQuasar();
const apiMethods = ["get", "post", "put"];
const outputFormats = ["json", "ndjson"];
const store = useStore();
const { t } = useI18n();
const formData: Ref<DestinationData> = ref({
  name: "",
  url: "",
  method: "post",
  skip_tls_verify: false,
  template: "",
  headers: {},
  emails: "",
  type: "http",
  action_id: "",
  output_format: "json",
});
const isUpdatingDestination = ref(false);

const isLoadingActions = ref(false);

const router = useRouter();

const actionOptions = ref<{ value: string; label: string; type: string }[]>([]);

const filteredActions = ref<any[]>([]);

const { getAllActions } = useActions();

// TODO OK: Use UUID package instead of this and move this method in utils
const getUUID = () => {
  return (Math.floor(Math.random() * (9999999999 - 100 + 1)) + 100).toString();
};

const apiHeaders: Ref<
  {
    key: string;
    value: string;
    uuid: string;
  }[]
> = ref([{ key: "", value: "", uuid: getUUID() }]);

const tabs = computed(() => {
  let tabs = [
    {
      label: t("alerts.webhook"),
      value: "http",
      style: {
        width: "fit-content",
        padding: "4px 14px",
        background: formData.value.type === "http" ? "#5960B2" : "",
        border: "none !important",
        color: formData.value.type === "http" ? "#ffffff !important" : "",
      },
    },
    {
      label: t("alerts.email"),
      value: "email",
      style: {
        width: "fit-content",
        padding: "4px 14px",
        background: formData.value.type === "email" ? "#5960B2" : "",
        border: "none !important",
        color: formData.value.type === "email" ? "#ffffff !important" : "",
      },
    },
  ];

  if (
    (config.isEnterprise == "true" || config.isCloud == "true") &&
    store.state.zoConfig.actions_enabled
  ) {
    tabs.push({
      label: t("alerts.action"),
      value: "action",
      style: {
        width: "fit-content",
        padding: "4px 14px",
        background: formData.value.type === "action" ? "#5960B2" : "",
        border: "none !important",
        color: formData.value.type === "action" ? "#ffffff !important" : "",
      },
    });
  }

  return tabs;
});

onActivated(() => setupDestinationData());
onBeforeMount(async () => {
  setupDestinationData();
  await getActionOptions();
});

const setupDestinationData = () => {
  if (props.destination) {
    isUpdatingDestination.value = true;
    formData.value.name = props.destination.name;
    formData.value.url = props.destination.url;
    formData.value.method = props.destination.method;
    formData.value.skip_tls_verify = props.destination.skip_tls_verify;
    formData.value.template = props.destination.template;
    if (!props.destination.headers) formData.value.headers = {};
    formData.value.headers = props.destination.headers;
    formData.value.emails = (props.destination?.emails || []).join(", ");
    formData.value.type = props.destination.type || "http";
    formData.value.action_id = props.destination.action_id || "";

    if (Object.keys(formData.value?.headers || {}).length) {
      apiHeaders.value = [];
      Object.entries(formData.value?.headers || {}).forEach(([key, value]) => {
        addApiHeader(key, value);
      });
    }
    if (props.destination.output_format) {
      formData.value.output_format = props.destination.output_format;
    }
  }
};

const getFormattedTemplates = computed(() =>
  props.templates
    .filter((template: any) => {
      if (formData.value.type === "email" && template.type === "email")
        return true;
      else if (formData.value.type !== "email") return true;
    })
    .map((template: any) => template.name),
);

const isValidDestination = computed(
  () =>
    formData.value.name &&
    ((formData.value.url &&
      formData.value.method &&
      formData.value.type === "http") ||
      (formData.value.type === "email" && formData.value?.emails?.length) ||
      (formData.value.type === "action" && formData.value?.action_id?.length) ||
      (!props.isAlerts && formData.value.url && formData?.value?.method)) &&
    (props.isAlerts ? formData.value.template : true),
);

const updateActionOptions = () => {
  actionOptions.value = [];
  store.state.organizationData.actions.forEach((action: any) => {
    if (action.execution_details_type === "service")
      actionOptions.value.push({
        value: action.id,
        label: action.name,
        type: action.execution_details_type,
      });
  });
  filteredActions.value = actionOptions.value;
};

const getActionOptions = async () => {
  try {
    isLoadingActions.value = true;
    // Update action options with existing actions
    updateActionOptions();

    // Get all actions from the server and update the action options
    await getAllActions();
    isLoadingActions.value = false;
    updateActionOptions();
  } catch (err) {
    console.error(err);
  } finally {
    isLoadingActions.value = false;
  }
};

const saveDestination = () => {
  if (!isValidDestination.value) {
    q.notify({
      type: "negative",
      message: "Please fill required fields",
      timeout: 1500,
    });
    return;
  }
  const dismiss = q.notify({
    spinner: true,
    message: "Please wait...",
    timeout: 2000,
  });
  const headers: Headers = {};
  apiHeaders.value.forEach((header) => {
    if (header["key"] && header["value"]) headers[header.key] = header.value;
  });

  const payload: any = {
    url: formData.value.url,
    method: formData.value.method,
    skip_tls_verify: formData.value.skip_tls_verify,
    template: props.isAlerts ? formData.value.template : "",
    headers: headers,
    name: formData.value.name,
    
  };

  if(!props.isAlerts){
    payload["output_format"] = formData.value.output_format;
  }

  if (formData.value.type === "email") {
    payload["type"] = "email";
    payload["emails"] = (formData.value?.emails || "")
      .split(/[;,]/)
      .map((email: string) => email.trim());
  }

  if (formData.value.type === "action") {
    payload["type"] = "action";
    payload["action_id"] = formData.value.action_id;
  }

  // if (!props.isAlerts) {
  //   payload["type"] = "remote_pipeline";
  // }

  if (isUpdatingDestination.value) {
    destinationService
      .update({
        org_identifier: store.state.selectedOrganization.identifier,
        destination_name: formData.value.name,
        data: payload,
      })
      .then(() => {
        dismiss();
        emit("get:destinations");
        emit("cancel:hideform");
        q.notify({
          type: "positive",
          message: `Destination saved successfully.`,
        });
      })
      .catch((err: any) => {
        if (err.response?.status == 403) {
          return;
        }
        dismiss();
        q.notify({
          type: "negative",
          message: err.response?.data?.error || err.response?.data?.message,
        });
      });
  } else {
    destinationService
      .create({
        org_identifier: store.state.selectedOrganization.identifier,
        destination_name: formData.value.name,
        data: payload,
      })
      .then(() => {
        dismiss();
        emit("get:destinations");
        emit("cancel:hideform");
        q.notify({
          type: "positive",
          message: `Destination saved successfully.`,
        });
      })
      .catch((err: any) => {
        if (err.response?.status == 403) {
          return;
        }
        dismiss();
        q.notify({
          type: "negative",
          message: err.response?.data?.error || err.response?.data?.message,
        });
      });
  }
};
const addApiHeader = (key: string = "", value: string = "") => {
  apiHeaders.value.push({ key: key, value: value, uuid: getUUID() });
};
const deleteApiHeader = (header: any) => {
  apiHeaders.value = apiHeaders.value.filter(
    (_header) => _header.uuid !== header.uuid,
  );
  if (formData.value?.headers?.[header.key])
    delete formData.value?.headers?.[header.key];
  if (!apiHeaders.value.length) addApiHeader();
};

const createEmailTemplate = () => {
  router.push({
    name: "alertTemplates",
    query: {
      action: "add",
      type: "email",
      org_identifier: store.state.selectedOrganization.identifier,
    },
  });
};

const filterColumns = (options: any[], val: String, update: Function) => {
  let filteredOptions: any[] = [];
  if (val === "") {
    update(() => {
      filteredOptions = [...options];
    });
    return filteredOptions;
  }
  update(() => {
    const value = val?.toLowerCase();
    filteredOptions = options.filter((column: any) => {
      if (typeof column === "string")
        return column?.toLowerCase().indexOf(value) > -1;
      else {
        return column?.label?.toLowerCase().indexOf(value) > -1;
      }
    });
  });
  return filteredOptions;
};

const filterActions = (val: string, update: any) => {
  filteredActions.value = filterColumns(actionOptions.value, val, update);
};
</script>
<style lang="scss" scoped>
#editor {
  width: 100%;
  min-height: 5rem;
  padding-bottom: 14px;
  resize: both;
}

.page-content {
  height: calc(100vh - 112px);
}
</style>
<style lang="scss">
.destination-method-select {
  .q-field__native > :first-child {
    text-transform: uppercase !important;
  }
}

.no-case .q-field__native span {
  text-transform: none !important;
}
</style>
