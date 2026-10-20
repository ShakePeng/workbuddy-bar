import { createApp } from "vue";
import App from "./App.vue";
import { applyTheme } from "./lib/theme";
import "./style.css";

applyTheme("system");
createApp(App).mount("#app");
