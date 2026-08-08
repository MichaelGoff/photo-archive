import { createApp } from 'vue';
import App from './App.vue';
import { createVuetify } from 'vuetify';
import { createPinia } from 'pinia';
import 'unfonts.css';
import 'vuetify/styles';
import { aliases, mdi } from 'vuetify/iconsets/mdi-svg';

const vuetify = createVuetify({
  icons: {
    defaultSet: 'mdi',
    aliases,
    sets: {
      mdi,
    },
  },
});

const pinia = createPinia();

createApp(App).use(vuetify).use(pinia).mount('#app');
