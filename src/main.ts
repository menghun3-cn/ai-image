import { createApp } from "vue";
import { createPinia } from "pinia";
import { createRouter, createWebHistory } from "vue-router";
import App from "./App.vue";
import "./assets/main.css";

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: "/",
      name: "generate",
      component: () => import("./views/GenerateView.vue"),
    },
    {
      path: "/gallery",
      name: "gallery",
      component: () => import("./views/GalleryView.vue"),
    },
    {
      path: "/settings",
      name: "settings",
      component: () => import("./views/SettingsView.vue"),
    },
  ],
});

const app = createApp(App);
app.use(createPinia());
app.use(router);
app.mount("#app");
