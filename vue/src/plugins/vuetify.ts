import Vue from "vue";
import Vuetify from "vuetify/lib/framework";

Vue.use(Vuetify);

export default new Vuetify({
  theme: {
    dark: true,
    themes: {
      dark: {
        primary: "#2196F3",
      },
      light: {
        primary: "#2196F3",
      },
    },
  },
});
