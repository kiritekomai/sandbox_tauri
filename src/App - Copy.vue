<script setup>
import { ref, computed } from "vue";
import { shallowRef } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
const debugMessage = ref("");
const tree = ref([]);
const selectedIds = ref([]);
const openIds = ref([]);
const currentFilePath = ref(""); // set when load
const newFileName = ref("newfile");
const selected = shallowRef([]);

function removeEmptyChildren(nodes) {
  return nodes.map((node) => {
    const newNode = { ...node };

    // 再帰的に子を処理
    if (Array.isArray(newNode.children)) {
      newNode.children = removeEmptyChildren(newNode.children);
      // 空になったら削除
      if (newNode.children.length === 0) {
        delete newNode.children;
      }
    }

    return newNode;
  });
}

function replaceIdsWithIntegers(items) {
  let counter = 1;

  function traverse(nodes) {
    return nodes.map((node) => {
      const newNode = { ...node, id: counter++ };
      if (node.children && node.children.length > 0) {
        newNode.children = traverse(node.children);
      }
      return newNode;
    });
  }

  return traverse(items);
}

function getAllParentNodeIds(nodes) {
  let ids = [];
  for (const node of nodes) {
    if (node.children && node.children.length > 0) {
      ids.push(node.id);
      ids = ids.concat(getAllParentNodeIds(node.children));
    }
  }
  return ids;
}

async function onLoadExample() {
  // Example: ask backend to load a file path. Adjust path as needed (absolute).
  const path = "C:/Users/Admin/Desktop/script/aaaa/sample.xml"; // <-- change to actual path on your machine
  currentFilePath.value = path;
  await invoke("load_files", { paths: [path] });
  const t = await invoke("get_tree", { path });
  tree.value = replaceIdsWithIntegers(removeEmptyChildren(t));
  openIds.value = getAllParentNodeIds(tree.value);
  debugMessage.value = openIds.value;
}

const hasGroupSelection = computed(() => {
  // For simplicity allow add/sort if any selection
  return selectedIds.value.length > 0;
});
const hasFileSelection = computed(() => selectedIds.value.length > 0);

async function doSort() {
  // invoke sort_groups with selectedIds
  await invoke("sort_groups", { node_ids: selectedIds.value, ascending: true });
  // refresh tree
  tree.value = await invoke("get_tree", { path: currentFilePath.value });
}

async function doAdd() {
  await invoke("add_file_to_groups", {
    node_ids: selectedIds.value,
    file_name: newFileName.value,
  });
  tree.value = await invoke("get_tree", { path: currentFilePath.value });
}

async function doDelete() {
  await invoke("delete_file_nodes", { node_ids: selectedIds.value });
  tree.value = await invoke("get_tree", { path: currentFilePath.value });
}

async function doSave() {
  await invoke("save_file", { path: currentFilePath.value });
  // optionally notify saved
  alert("Saved");
}

async function printDebug() {
  // debugMessage.value = tree.value;
  debugMessage.value = tree.value;
}
</script>

<template>
  <main class="container">
    <v-container>
      <v-btn @click="onLoadExample">Load sample.xml</v-btn>
      <!-- <v-treeview :items="items" item-value="id" item-props selectable></v-treeview> -->
      <v-treeview
        :items="tree"
        select-strategy="leaf"
        item-key="id"
        item-title="label"
        item-children="children"
        indent-lines="default"
        open-on-click
        selectable
      >
      </v-treeview>
      <v-btn @click="doAdd" :disabled="!hasGroupSelection">Add</v-btn>
      <v-btn @click="doSort" :disabled="!hasGroupSelection">Sort</v-btn>
      <v-btn @click="doDelete" :disabled="!hasFileSelection">Delete</v-btn>
      <v-btn @click="doSave" :disabled="!currentFilePath">Save</v-btn>
    </v-container>
    <v-btn @click="printDebug">print debug</v-btn>
    <p>{{ debugMessage }}</p>
  </main>
</template>

<style>
:root {
  font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
  font-size: 16px;
  line-height: 24px;
  font-weight: 400;

  color: #0f0f0f;
  background-color: #f6f6f6;

  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  -webkit-text-size-adjust: 100%;
}

.container {
  margin: 0;
  padding-top: 10vh;
  display: flex;
  flex-direction: column;
  justify-content: center;
  text-align: left;
}

.row {
  display: flex;
  justify-content: center;
}

a {
  font-weight: 500;
  color: #646cff;
  text-decoration: inherit;
}

a:hover {
  color: #535bf2;
}

h1 {
  text-align: center;
}

input,
button {
  border-radius: 8px;
  border: 1px solid transparent;
  padding: 0.6em 1.2em;
  font-size: 1em;
  font-weight: 500;
  font-family: inherit;
  color: #0f0f0f;
  background-color: #ffffff;
  transition: border-color 0.25s;
  box-shadow: 0 2px 2px rgba(0, 0, 0, 0.2);
}

button {
  cursor: pointer;
}

button:hover {
  border-color: #396cd8;
}
button:active {
  border-color: #396cd8;
  background-color: #e8e8e8;
}

input,
button {
  outline: none;
}

#greet-input {
  margin-right: 5px;
}

@media (prefers-color-scheme: dark) {
  :root {
    color: #f6f6f6;
    background-color: #2f2f2f;
  }

  a:hover {
    color: #24c8db;
  }

  input,
  button {
    color: #ffffff;
    background-color: #0f0f0f98;
  }
  button:active {
    background-color: #0f0f0f69;
  }
}
</style>
