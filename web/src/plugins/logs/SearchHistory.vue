<template>
  <div
    :class="
      store.state.theme === 'dark'
        ? 'dark-theme-history-page'
        : 'light-theme-history-page'
    "
  >
    <div class="flex tw-justify-between tw-items-center">
      <div class="flex items-center q-py-sm q-pl-md">
        <div
          data-test="search-history-alert-back-btn"
          class="flex justify-center items-center q-mr-md cursor-pointer"
          style="
            border: 1.5px solid;
            border-radius: 50%;
            width: 22px;
            height: 22px;
          "
          title="Go Back"
          @click="closeSearchHistory"
        >
          <q-icon name="arrow_back_ios_new" size="14px" />
        </div>
        <div class="text-h6" data-test="add-alert-title">Search History</div>
      </div>
      <div class="flex items-center q-py-sm q-pr-md">
        <div>
          <q-toggle v-model="wrapText" label="Wrap Text" class="q-mr-md" />
        </div>
        <div class="warning-text flex items-center q-py-xs q-px-sm q-mr-md">
          <q-icon name="info" class="q-mr-xs" size="16px" />
          <div>
            Search History might be delayed by <b> {{ delayMessage }}</b>
          </div>
        </div>
        <date-time
          data-test-name="search-history-date-time"
          ref="searchDateTimeRef"
          auto-apply
          :default-type="searchObj.data.datetime.type"
          @on:date-change="updateDateTime"
        />
        <div>
          <q-btn
            color="secondary"
            label="Get History"
            @click="fetchSearchHistory"
            class="q-ml-md"
            :disable="isLoading"
          />
        </div>
      </div>
    </div>

    <div class="">
      <q-page>
        <q-table
          ref="qTable"
          dense
          :rows="dataToBeLoaded"
          :columns="columnsToBeRendered"
          :pagination.sync="pagination"
          row-key="trace_id"
          :rows-per-page-options="[]"
          class="custom-table"
          :sort-method="sortMethod"
          :wrap-cells="wrapText"
        >
          <template v-slot:body="props">
            <q-tr
              :data-test="`stream-association-table-${props.row.trace_id}-row`"
              :props="props"
              style="cursor: pointer"
              @click="triggerExpand(props)"
            >
              <q-td>
                <q-btn
                  dense
                  flat
                  size="xs"
                  :icon="
                    expandedRow != props.row.uuid
                      ? 'expand_more'
                      : 'expand_less'
                  "
                />
              </q-td>

              <q-td
                :style="{
                  whiteSpace:
                    wrapText && col.name === 'sql' ? 'wrap' : 'nowrap',
                }"
                v-for="col in columnsToBeRendered.slice(1)"
                :key="col.name"
                :props="props"
              >
                {{ props.row[col.field] }}
              </q-td>
            </q-tr>
            <q-tr v-show="expandedRow === props.row.uuid" :props="props">
              <q-td colspan="100%">
                <div class="app-tabs-schedule-list report-list-tabs">
                  <app-tabs
                    data-test="expanded-list-tabs"
                    class="q-mr-md"
                    :tabs="tabs"
                    v-model:active-tab="activeTab"
                  />
                </div>
                <div v-show="activeTab === 'query'">
                  <div class="text-left tw-px-2 q-mb-sm expanded-content">
                    <div class="tw-flex tw-items-center q-py-sm">
                      <strong
                        >SQL Query :
                        <span>
                          <q-btn
                            @click.stop="
                              copyToClipboard(props.row.sql, 'SQL Query')
                            "
                            size="xs"
                            dense
                            flat
                            icon="content_copy"
                            class="copy-btn-sql tw-ml-2 tw-py-2 tw-px-2" /></span
                      ></strong>
                      <q-btn
                        @click.stop="goToLogs(props.row)"
                        size="xs"
                        label="Logs"
                        dense
                        class="copy-btn tw-py-2 tw-mx-2 tw-px-2"
                        icon="search"
                        flat
                        style="
                          color: #f2452f;
                          border: #f2452f 1px solid;
                          font-weight: bolder;
                        "
                      />
                    </div>
                    <div class="tw-flex tw-items-start tw-justify-center">
                      <div class="scrollable-content expanded-sql">
                        <pre style="text-wrap: wrap">{{ props.row?.sql }}</pre>
                      </div>
                    </div>
                  </div>
                  <div
                    v-if="props.row?.function"
                    class="text-left q-mb-sm tw-px-2 expanded-content"
                  >
                    <div class="tw-flex tw-items-center q-py-sm">
                      <strong
                        >Function Definition :
                        <span>
                          <q-btn
                            @click.stop="
                              copyToClipboard(
                                props.row.function,
                                'Function Defination',
                              )
                            "
                            size="xs"
                            dense
                            flat
                            icon="content_copy"
                            class="copy-btn-function tw-ml-2 tw-py-2 tw-px-2" /></span
                      ></strong>
                    </div>

                    <div class="tw-flex tw-items-start tw-justify-center">
                      <div class="scrollable-content expanded-function">
                        <pre style="text-wrap: wrap">{{
                          props.row?.function
                        }}</pre>
                      </div>
                    </div>
                  </div>
                </div>
                <query-editor
                  v-show="activeTab === 'more_details'"
                  style="height: 200px"
                  :ref="`QueryEditorRef${props.row.trace_id + props.row.sql}`"
                  :editor-id="`search-query-editor${props.row.trace_id + props.row.sql}`"
                  class="monaco-editor"
                  :debounceTime="600"
                  v-model:query="moreDetailsToDisplay"
                  language="json"
                  read-only
                />
              </q-td>
            </q-tr>
          </template>
          <template #bottom="scope">
            <div class="tw-ml-auto tw-mr-2">Max Limit : <b>1000</b></div>
            <q-separator
              style="height: 1.5rem; margin: auto 0"
              vertical
              inset
              class="q-mr-md"
            />

            <div class="q-pl-md">
              <QTablePagination
                :scope="scope"
                :position="'bottom'"
                :resultTotal="resultTotal"
                :perPageOptions="perPageOptions"
                @update:changeRecordPerPage="changePagination"
              />
            </div>
          </template>
          <template #no-data>
            <div v-if="!isLoading" class="tw-flex tw-mx-auto">
              <NoData />
            </div>
          </template>
        </q-table>

        <div
          v-if="isLoading"
          class="text-center full-width full-height q-mt-lg tw-flex tw-justify-center"
        >
          <q-spinner-hourglass color="primary" size="lg" />
        </div>
      </q-page>
    </div>
  </div>

  <!-- Show NoData component if there's no data to display -->
</template>
<script lang="ts">
//@ts-nocheck
import { ref, watch, onMounted, nextTick, computed, onUnmounted } from "vue";
import {
  timestampToTimezoneDate,
  b64EncodeUnicode,
  convertDateToTimestamp,
  getUUID,
} from "@/utils/zincutils";
import { useRouter, useRoute } from "vue-router";
import { useStore } from "vuex";
import { defineAsyncComponent, defineComponent } from "vue";
import useLogs from "../../composables/useLogs";
import TenstackTable from "../../plugins/logs/TenstackTable.vue";
import searchService from "@/services/search";
import NoData from "@/components/shared/grid/NoData.vue";
import DateTime from "@/components/DateTime.vue";
import { useI18n } from "vue-i18n";
import { date, QTable, useQuasar } from "quasar";
import type { Ref } from "vue";
import QTablePagination from "@/components/shared/grid/Pagination.vue";
import AppTabs from "@/components/common/AppTabs.vue";

const QueryEditor = defineAsyncComponent(
  () => import("@/components/CodeQueryEditor.vue"),
);

export default defineComponent({
  name: "SearchHistoryComponent",
  components: {
    DateTime,
    NoData,
    QTablePagination,
    AppTabs,
    QueryEditor,
  },
  props: {
    isClicked: {
      type: Boolean,
      default: false,
    },
  },
  emits: ["closeSearchHistory"],
  methods: {
    closeSearchHistory() {
      this.$emit("closeSearchHistory");
    },
  },
  setup(props, { emit }) {
    const router = useRouter();
    const $q = useQuasar();
    const route = useRoute();
    const store = useStore();
    const { t } = useI18n();
    const qTable: Ref<InstanceType<typeof QTable> | null> = ref(null);
    const searchDateTimeRef = ref(null);
    const wrapText = ref(true);
    const { searchObj, extractTimestamps } = useLogs();
    const dataToBeLoaded: any = ref([]);
    const dateTimeToBeSent = ref({
      valueType: "relative",
      relativeTimePeriod: "15m",
      startTime: 0,
      endTime: 0,
    });
    const columnsToBeRendered = ref([]);
    const expandedRow = ref([]); // Array to track expanded rows
    const isLoading = ref(false);
    const isDateTimeChanged = ref(false);
    const moreDetailsToDisplay = ref("");

    const activeTab = ref("query");
    const tabs = ref([
      {
        label: "Query / Function",
        value: "query",
      },
      {
        label: "More Details",
        value: "more_details",
      },
    ]);

    onUnmounted(() => {});

    const perPageOptions: any = [
      { label: "5", value: 5 },
      { label: "10", value: 10 },
      { label: "20", value: 20 },
      { label: "50", value: 50 },
      { label: "100", value: 100 },
      { label: "All", value: 0 },
    ];

    const resultTotal = ref<number>(0);

    const pagination = ref({
      page: 1,
      rowsPerPage: 100,
    });
    const selectedPerPage = ref(pagination.value.rowsPerPage);

    const generateColumns = (data: any) => {
      if (data.length === 0) return [];

      // Define the desired column order and names
      const desiredColumns = [
        { key: "#", label: "#" },
        { key: "executed_time", label: "Executed At" },

        { key: "sql", label: "SQL Query" },
      ];
      let aligin = "left";

      return desiredColumns.map(({ key, label }) => {
        if (key == "sql") {
          aligin = "left";
        }
        // Custom width for each column
        return {
          name: key, // Field name
          label: label, // Column label
          field: key, // Field accessor
          align: aligin,
          sortable: true,
        };
      });
    };

    const fetchSearchHistory = async () => {
      columnsToBeRendered.value = [];
      dataToBeLoaded.value = [];
      expandedRow.value = [];
      try {
        const { org_identifier } = router.currentRoute.value.query;
        isLoading.value = true;
        if (dateTimeToBeSent.value.valueType === "relative") {
          const convertedData = extractTimestamps(
            dateTimeToBeSent.value.relativeTimePeriod,
          );
          dateTimeToBeSent.value.startTime = convertedData.from * 1000;
          dateTimeToBeSent.value.endTime = convertedData.to * 1000;
        }
        const { startTime, endTime } = dateTimeToBeSent.value;
        const response = await searchService.get_history(
          org_identifier,
          startTime,
          endTime,
        );
        const limitedHits = response.data.hits;
        const filteredHits = limitedHits.filter(
          (hit) => hit.event === "Search",
        );
        if (filteredHits.length > 0) {
          resultTotal.value = filteredHits.length;
        }
        columnsToBeRendered.value = generateColumns(filteredHits);
        filteredHits.forEach((hit: any) => {
          //adding uuid to each which will be used to track the expanded row
          //why not trace_id ? because trace_id is not unique for each hit
          //and it can be same for multiple hits
          hit.uuid = getUUID();
          const { formatted, raw } = calculateDuration(
            hit.start_time,
            hit.end_time,
          );
          hit.duration = formatted;
          hit.rawDuration = raw;
          hit.toBeStoredStartTime = hit.start_time;
          hit.toBeStoredEndTime = hit.end_time;
          hit.start_time = timestampToTimezoneDate(
            hit.start_time / 1000,
            store.state.timezone,
            "yyyy-MM-dd HH:mm:ss.SSS",
          );
          hit.end_time = timestampToTimezoneDate(
            hit.end_time / 1000,
            store.state.timezone,
            "yyyy-MM-dd HH:mm:ss.SSS",
          );
          hit.rawTook = hit.took;
          hit.took = formatTime(hit.took);
          hit.rawScanRecords = hit.scan_records;
          hit.scan_records = hit.scan_records;
          hit.rawScanSize = hit.scan_size;
          hit.scan_size = hit.scan_size + hit.unit;
          hit.cached_ratio = hit.cached_ratio;
          hit.rawCachedRatio = hit.cached_ratio;
          hit.sql = hit.sql;
          hit.function = hit.function;
          hit.rawExecutedTime = hit._timestamp;
          hit.executed_time = timestampToTimezoneDate(
            hit._timestamp / 1000,
            store.state.timezone,
            "yyyy-MM-dd HH:mm:ss.SSS",
          );
        });
        dataToBeLoaded.value = filteredHits;
        isLoading.value = false;
      } catch (error) {
        $q.notify({
          type: "negative",
          message: "Failed to fetch search history. Please try again later.",
          timeout: 5000,
        });
        console.log(error, "error");
        isLoading.value = false;
      } finally {
        isLoading.value = false;
      }
    };
    //this method needs to revamped / can be made shorter
    const sortMethod = (rows, sortBy, descending) => {
      const data = [...rows];
      if (sortBy === "duration") {
        if (descending) {
          return data.sort((a, b) => b.rawDuration - a.rawDuration);
        }
        return data.sort((a, b) => a.rawDuration - b.rawDuration);
      }

      if (sortBy === "took") {
        if (descending) {
          return data.sort((a, b) => b.rawTook - a.rawTook);
        }
        return data.sort((a, b) => a.rawTook - b.rawTook);
      }
      if (sortBy === "scan_records") {
        if (descending) {
          return data.sort((a, b) => b.rawScanRecords - a.rawScanRecords);
        }
        // console.log(data.sort((a, b) => a.rawScanRecords - b.rawScanRecords), "data")
        return data.sort((a, b) => a.rawScanRecords - b.rawScanRecords);
      }
      if (sortBy === "scan_size") {
        if (descending) {
          return data.sort((a, b) => b.rawScanSize - a.rawScanSize);
        }
        // console.log(data.sort((a, b) => a.rawScanRecords - b.rawScanRecords), "data")
        return data.sort((a, b) => a.rawScanSize - b.rawScanSize);
      }
      if (sortBy === "cached_ratio") {
        if (descending) {
          return data.sort((a, b) => b.rawCachedRatio - a.rawCachedRatio);
        }
        // console.log(data.sort((a, b) => a.rawScanRecords - b.rawScanRecords), "data")
        return data.sort((a, b) => a.rawCachedRatio - b.rawCachedRatio);
      }
      if (sortBy == "start_time") {
        if (descending) {
          return data.sort(
            (a, b) => b.toBeStoredStartTime - a.toBeStoredStartTime,
          );
        }
        return data.sort(
          (a, b) => a.toBeStoredStartTime - b.toBeStoredStartTime,
        );
      }

      if (sortBy == "end_time") {
        if (descending) {
          return data.sort((a, b) => b.toBeStoredEndTime - a.toBeStoredEndTime);
        }
        return data.sort((a, b) => a.toBeStoredEndTime - b.toBeStoredEndTime);
      }
      if (sortBy == "executed_time") {
        if (descending) {
          return data.sort((a, b) => b.rawExecutedTime - a.rawExecutedTime);
        }
        return data.sort((a, b) => a.rawExecutedTime - b.rawExecutedTime);
      }
    };
    const copyToClipboard = (text, type) => {
      navigator.clipboard
        .writeText(text)
        .then(() => {
          $q.notify({
            type: "positive",
            message: `${type} Copied Successfully!`,
            timeout: 5000,
          });
        })
        .catch(() => {
          $q.notify({
            type: "negative",
            message: "Error while copy content.",
            timeout: 5000,
          });
        });
    };
    const delayMessage = computed(() => {
      const delay = store.state.zoConfig.usage_publish_interval;
      if (delay <= 60) {
        return "60 seconds";
      } else {
        const minutes = Math.floor(delay / 60);
        return `${minutes} minute(s)`;
      }
    });

    const updateDateTime = async (value: any) => {
      const { startTime, endTime } = value;
      dateTimeToBeSent.value = value;
      searchDateTimeRef.value.setAbsoluteTime(value.startTime, value.endTime);
    };
    const formatTime = (took) => {
      return `${took.toFixed(2)} sec`;
    };
    const calculateDuration = (startTime, endTime) => {
      const durationMicroseconds = endTime - startTime;
      const durationSeconds = durationMicroseconds / 1e6;

      // Store the raw duration in a separate property
      const rawDuration = durationSeconds;

      let result = "";

      if (durationSeconds < 60) {
        result = `${durationSeconds.toFixed(2)} seconds`;
      } else if (durationSeconds < 3600) {
        const minutes = Math.floor(durationSeconds / 60);
        const seconds = durationSeconds % 60;
        result = `${minutes} minutes`;
        if (seconds > 0) {
          result += ` and ${seconds.toFixed(2)} seconds`;
        }
      } else if (durationSeconds < 86400) {
        const hours = Math.floor(durationSeconds / 3600);
        const minutes = Math.floor((durationSeconds % 3600) / 60);
        result = `${hours} hours`;
        if (minutes > 0) {
          result += ` and ${minutes} minutes`;
        }
      } else if (durationSeconds < 2592000) {
        const days = Math.floor(durationSeconds / 86400);
        const hours = Math.floor((durationSeconds % 86400) / 3600);
        result = `${days} days`;
        if (hours > 0) {
          result += ` and ${hours} hours`;
        }
      } else if (durationSeconds < 31536000) {
        const months = Math.floor(durationSeconds / 2592000);
        const days = Math.floor((durationSeconds % 2592000) / 86400);
        result = `${months} months`;
        if (days > 0) {
          result += ` and ${days} days`;
        }
      } else {
        const years = Math.floor(durationSeconds / 31536000);
        const months = Math.floor((durationSeconds % 31536000) / 2592000);
        result = `${years} years`;
        if (months > 0) {
          result += ` and ${months} months`;
        }
      }

      return { formatted: result, raw: rawDuration };
    };

    const triggerExpand = (props) => {
      moreDetailsToDisplay.value = JSON.stringify(
        filterRow(props.row),
        null,
        2,
      );
      if (expandedRow.value === props.row.uuid) {
        expandedRow.value = null;
      } else {
        // Otherwise, expand the clicked row and collapse any other row
        expandedRow.value = props.row.uuid;
      }
    };
    const goToLogs = (row) => {
      const duration_suffix = row.duration.split(" ")[1];
      // emit('closeSearchHistory');
      const stream: string = row.stream_name;
      const from = row.toBeStoredStartTime;
      const to = row.toBeStoredEndTime;
      const refresh = 0;

      const query = b64EncodeUnicode(row.sql);

      const queryObject = {
        stream_type: "logs",
        stream,
        period: "15m",
        refresh,
        sql_mode: "true",
        query,
        defined_schemas: "user_defined_schema",
        org_identifier: row.org_id,
        quick_mode: "false",
        show_histogram: "true",
        type: "search_history_re_apply",
      };

      if (row.hasOwnProperty("function") && row.function) {
        const functionContent = b64EncodeUnicode(row.function);
        queryObject["functionContent"] = functionContent;
      }

      router.push({
        path: "/logs",
        query: queryObject,
      });
    };
    const changePagination = (val: { label: string; value: any }) => {
      if (val.label == "All") {
        val.value = dataToBeLoaded.value.length;
        val.label = "All";
      }
      selectedPerPage.value = val.value;
      pagination.value.rowsPerPage = val.value;
      qTable.value.setPagination(pagination.value);

      // pagination.value.page = 1;
    };

    watch(
      () => props.isClicked,
      (value) => {
        if (value == true && !isLoading.value) {
          fetchSearchHistory();
        }
      },
    );

    function filterRow(row) {
      const desiredColumns = [
        { key: "trace_id", label: "Trace ID" },
        { key: "start_time", label: "Start Time" },
        { key: "end_time", label: "End Time" },
        { key: "duration", label: "Duration" },
        { key: "took", label: "Took" },
        { key: "scan_size", label: "Scan Size" },
        { key: "scan_records", label: "Scan Records" },
        { key: "cached_ratio", label: "Cached Ratio" },
      ];
      return desiredColumns.reduce((filtered, column) => {
        if (row[column.key] !== undefined) {
          filtered[column.key] = row[column.key];
        }
        return filtered;
      }, {});
    }
    return {
      searchObj,
      store,
      generateColumns,
      fetchSearchHistory,
      dataToBeLoaded,
      columnsToBeRendered,
      t,
      route,
      isLoading,
      qTable,
      updateDateTime,
      pagination,
      searchDateTimeRef,
      expandedRow,
      goToLogs,
      triggerExpand,
      copyToClipboard,
      formatTime,
      delayMessage,
      sortMethod,
      resultTotal,
      perPageOptions,
      changePagination,
      selectedPerPage,
      activeTab,
      tabs,
      moreDetailsToDisplay,
      wrapText,
    };
    // Watch the searchObj for changes
  },
});
</script>
<style lang="scss" scoped>
.expanded-content {
  padding: 0 0.5rem 0rem 1rem;
  width: calc(95vw - 40px);
  max-height: 100vh; /* Set a fixed height for the container */
  overflow: hidden; /* Hide overflow by default */
}

.scrollable-content {
  width: 100%; /* Use the full width of the parent */
  overflow-y: auto; /* Enable vertical scrolling for long content */
  padding: 10px; /* Optional: padding for aesthetics */
  border: 1px solid #ddd; /* Optional: border for visibility */
  height: 100%;
  max-height: 200px;
  /* Use the full height of the parent */
  text-wrap: normal;
  background-color: #e8e8e8;
  color: black;
}

.q-td {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.custom-table .q-tr > .q-td:nth-child(2) {
  text-align: left;
}

.copy-btn-sql {
  border: #7a54a2 1px solid;
  color: #7a54a2;
}

.copy-btn-function {
  border: #0a7ebc 1px solid;
  color: #0a7ebc;
}

.warning-text {
  color: #f5a623;
  border: 1px solid #f5a623;
  border-radius: 2px;
}
.expanded-sql {
  border-left: #7a54a2 3px solid;
}
.expanded-function {
  border-left: #0a7ebc 3px solid;
}

.report-list-tabs {
  height: fit-content;

  :deep(.rum-tabs) {
    border: 1px solid #464646;
  }

  :deep(.rum-tab) {
    &:hover {
      background: #464646;
    }

    &.active {
      background: #5960b2;
      color: #ffffff !important;
    }
  }
}

.report-list-tabs {
  padding: 0 1rem;
  height: fit-content;
  width: fit-content;

  :deep(.rum-tabs) {
    border: 1px solid #eaeaea;
    height: fit-content;
    border-radius: 4px;
    overflow: hidden;
  }

  :deep(.rum-tab) {
    width: fit-content !important;
    padding: 4px 12px !important;
    border: none !important;

    &:hover {
      background: #eaeaea;
    }

    &.active {
      background: #5960b2;
      color: #ffffff !important;
    }
  }
}
</style>
