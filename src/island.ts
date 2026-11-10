import { createApp } from "vue";
import { MotionPlugin } from "@vueuse/motion";
import IslandApp from "./IslandApp.vue";
import "./style.css";

createApp(IslandApp).use(MotionPlugin).mount("#app");
