import { defineConfig } from 'vitepress'

export default defineConfig({
  title: 'vz',
  description: 'Zero-config terminal data visualization',
  base: '/vz/',
  appearance: 'dark',
  head: [
    ['link', { rel: 'icon', href: '/vz/favicon.svg' }],
    ['meta', { property: 'og:title', content: 'vz — Zero-Config Terminal Data Visualization' }],
    ['meta', { property: 'og:description', content: 'CLI BI tool that auto-visualizes CSV, JSON, TSV data in your terminal.' }],
    ['meta', { property: 'og:type', content: 'website' }],
    ['meta', { name: 'twitter:card', content: 'summary' }],
  ],
  locales: {
    root: {
      label: 'English',
      lang: 'en',
      themeConfig: {
        nav: [
          { text: 'Guide', link: '/guide/getting-started' },
          { text: 'Demo', link: '/demo' },
        ],
        sidebar: [
          {
            text: 'Guide',
            items: [
              { text: 'Getting Started', link: '/guide/getting-started' },
              { text: 'Chart Types', link: '/guide/chart-types' },
              { text: 'Output Modes', link: '/guide/output-modes' },
            ]
          }
        ],
      }
    },
    ja: {
      label: '日本語',
      lang: 'ja',
      description: 'ゼロコンフィグのターミナルデータ可視化ツール',
      themeConfig: {
        nav: [
          { text: 'ガイド', link: '/ja/guide/getting-started' },
          { text: 'デモ', link: '/ja/demo' },
        ],
        sidebar: [
          {
            text: 'ガイド',
            items: [
              { text: 'はじめに', link: '/ja/guide/getting-started' },
              { text: 'チャート種別', link: '/ja/guide/chart-types' },
              { text: '出力モード', link: '/ja/guide/output-modes' },
            ]
          }
        ],
        editLink: {
          pattern: 'https://github.com/r-hsnin/vz/edit/main/docs/:path',
          text: 'このページを編集する',
        },
        outline: { label: '目次' },
        docFooter: { prev: '前のページ', next: '次のページ' },
        lastUpdated: { text: '最終更新' },
      }
    }
  },
  themeConfig: {
    logo: '/logo.svg',
    socialLinks: [
      { icon: 'github', link: 'https://github.com/r-hsnin/vz' }
    ],
    footer: {
      message: 'Released under the MIT License.',
      copyright: 'Built with <a href="https://ratatui.rs">ratatui</a> 🦀'
    },
    search: {
      provider: 'local'
    },
    editLink: {
      pattern: 'https://github.com/r-hsnin/vz/edit/main/docs/:path'
    }
  }
})
