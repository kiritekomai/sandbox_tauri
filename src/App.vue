<script setup>
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import SelectButton from "primevue/selectbutton";
import InputText from "primevue/inputtext";

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
const currentFilePath = ref(""); // set when load
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

//
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
  // Example: ask backend to load a file path. Adjust path as needed (absolute).
  const path = "C:/Users/Admin/Desktop/script/aaaa/sample.xml"; // <-- change to actual path on your machine
  currentFilePath.value = path;
  await invoke("load_files", { paths: [path] });
  const t = await invoke("get_tree", { path });
  nodeArray.value = markDisabled(t);
  openIds.value = expandAll(t);
  debugMessage.value = t;
}

async function doSort() {
  // invoke sort_groups with selectedIds
  await invoke("sort_groups", { node_ids: selectedIds.value, ascending: true });
  // refresh tree
  nodeArray.value = await invoke("get_tree", { path: currentFilePath.value });
}

async function doAdd() {
  await invoke("add_file_to_groups", {
    node_ids: selectedIds.value,
    file_name: newFileName.value,
  });
  nodeArray.value = await invoke("get_tree", { path: currentFilePath.value });
}

async function doDelete() {
  await invoke("delete_file_nodes", { node_ids: selectedIds.value });
  nodeArray.value = await invoke("get_tree", { path: currentFilePath.value });
}

async function doSave() {
  await invoke("save_file", { path: currentFilePath.value });
  // optionally notify saved
  alert("Saved");
}

async function printDebug() {
  // debugMessage.value = tree.value;
  debugMessage.value = nodeArray.value;
}
const filePath = ref("/path/to/opened/file.txt");
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
    <Button outlined label="編集" @click="doAdd" />
    <Button outlined label="保存" @click="doSave" />
    <Button outlined label="開く" @click="doSave" />

    <pre class="mt-4">Selected: {{ selectedIds }}</pre>
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
