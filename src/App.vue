<script setup>
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import SelectButton from "primevue/selectbutton";
import InputText from "primevue/inputtext";

const filePath = ref("");

const editModes = [
  { label: "ファイル追加", value: "addFile" },
  { label: "ファイル削除", value: "deleteFile" },
  { label: "ソート", value: "sort" },
  { label: "インクルード追加", value: "addInclude" },
  { label: "インクルード削除", value: "deleteInclude" },
];
const selectedEditMode = ref(editModes[0].value);

const debugMessage = ref("");
const nodeArray = ref([]);
const selectedIds = ref({});
const openIds = ref({});
const newFileName = ref("newfile");

function expandAll(data) {
  const keys = {};
  const traverse = (node) => {
    keys[node.key] = true;
    if (node.children) {
      node.children.forEach(traverse);
    }
  };
  data.forEach(traverse);
  return keys;
}

function markDisabled(nodes) {
  return nodes.map((node) => {
    const newNode = { ...node };
    if (node.node_type !== "file") {
      newNode.selectable = false;
    }
    if (node.children) {
      newNode.children = markDisabled(node.children);
    }
    return newNode;
  });
}
async function onLoadExample() {
  // For debug
  const path = "C:/Users/Admin/Desktop/script/aaaa/sample.xml";
  filePath.value = path;
  await invoke("load_files", { paths: [path] });
  const t = await invoke("get_tree", { path });
  nodeArray.value = markDisabled(t);
  openIds.value = expandAll(t);
  selectedIds.value = {};
  debugMessage.value = t;
}

async function openFile() {
  const selected = await open({
    multiple: false,
    directory: false,
    filters: [
      {
        name: "uVision workspace file",
        extensions: ["txt", "xml"],
      },
      {
        name: "All files",
        extensions: ["*"],
      },
    ],
  });

  if (selected) {
    console.log("選択されたファイル:", selected);
    filePath.value = selected;
    await invoke("load_files", { paths: [selected] });
    const t = await invoke("get_tree", { path: selected });
    nodeArray.value = markDisabled(t);
    openIds.value = expandAll(t);
    selectedIds.value = {};
  } else {
    console.log("キャンセルされました");
  }
}
async function doEdit() {
  switch (selectedEditMode.value) {
    case "addFile":
      await invoke("add_file_to_groups", {
        node_ids: selectedIds.value,
        file_name: newFileName.value,
      });
      break;
    case "deleteFile":
      await invoke("delete_file_nodes", { node_ids: selectedIds.value });
      break;
    case "sort":
      await invoke("sort_groups", {
        node_ids: selectedIds.value,
        ascending: true,
      });
      break;
    case "addInclude":
      break;
    case "deleteInclude":
      break;
    default:
      break;
  }
}

async function doSave() {
  // await invoke("save_file", { path: filePath.value });
  // optionally notify saved
  alert("Saved");
}

async function printDebug() {
  // debugMessage.value = tree.value;
  debugMessage.value = selectedIds.value;
}
</script>

<template>
  <main class="container">
    <Button outlined label="..." @click="onLoadExample" />

    <p class="desc">uvmwsを選択して下さい</p>
    <div class="flex items-center gap-2">
      <!-- ファイルパス表示 -->
      <InputText outlined v-model="filePath" readonly class="flex-1" />

      <!-- 開くボタン -->
      <Button
        outlined
        icon="pi pi-folder-open"
        @click="openFile"
        tooltip="開く"
        class="p-button-outlined p-button-sm"
      />
    </div>
    <p class="desc">編集モードを選択して下さい</p>
    <SelectButton
      outlined
      v-model="selectedEditMode"
      :options="editModes"
      optionLabel="label"
      optionValue="value"
    />

    <p class="desc">編集対象を選択して下さい</p>
    <Tree
      :value="nodeArray"
      v-model:selectionKeys="selectedIds"
      selectionMode="checkbox"
      :expandedKeys="openIds"
      :propagateSelectionUp="false"
      :propagateSelectionDown="false"
      :filter="true"
      filterMode="lenient"
      class="mt-4"
    />
    <Button outlined label="編集" @click="doEdit" />
    <Button outlined label="保存" @click="doSave" />
    <Button outlined label="開く" @click="doSave" />

    <Button outlined label="Debug" @click="printDebug" />

    <!-- <pre class="mt-4">Selected: {{ selectedIds }}</pre> -->
    <p>{{ debugMessage }}</p>
  </main>
</template>

<style>
.box {
  border: 1px solid var(--p-primary-color);
  border-radius: 5px;
  padding: 0.2rem;
}
.desc {
  font-size: 0.85rem;
  color: var(--p-primary-color);
  margin-top: 0.5rem;
  margin-bottom: 0.5rem;
}
.divider {
  border: none;
  border-top: 1px solid var(--p-primary-color);
  margin: 1.5rem 0;
}
.flex {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}
.flex-1 {
  flex: 1;
}
</style>
