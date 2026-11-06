<script lang="ts" module>
  // One outline icon set, 24-unit grid, drawn at 16px with a 1.6 stroke.
  export const icons = {
    home: "M4 10.5 12 4l8 6.5V19a1 1 0 0 1-1 1h-4.5v-5.5h-5V20H5a1 1 0 0 1-1-1z",
    history: "M12 7.5V12l3 2M20.5 12a8.5 8.5 0 1 1-2.6-6.1M20.5 4.5v4h-4",
    notes: "M7 3.5h7.5L19 8v12.5H7a1.5 1.5 0 0 1-1.5-1.5V5A1.5 1.5 0 0 1 7 3.5zM14 3.5V8.5h5M9 13h7M9 16.5h5",
    dictionary: "M5 19.5V5.5A2 2 0 0 1 7 3.5h12v14H7a2 2 0 0 0-2 2zm0 0a2 2 0 0 0 2 2h12M9 8h6",
    snippets: "M9 7 4 12l5 5M15 7l5 5-5 5",
    styles: "M4.5 19.5h4l10-10-4-4-10 10zM13 7l4 4",
    settings: "M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM19 13.5v-3l-2.1-.5a5.6 5.6 0 0 0-.6-1.4l1.2-1.8-2.1-2.1-1.8 1.2a5.6 5.6 0 0 0-1.4-.6L11.7 3h-3l-.5 2.1a5.6 5.6 0 0 0-1.4.6L5 4.6 2.9 6.7l1.2 1.8a5.6 5.6 0 0 0-.6 1.4L1.4 10.5v3l2.1.5c.1.5.3 1 .6 1.4l-1.2 1.8L5 19.3l1.8-1.2c.4.3.9.5 1.4.6l.5 2.1h3l.5-2.1c.5-.1 1-.3 1.4-.6l1.8 1.2 2.1-2.1-1.2-1.8c.3-.4.5-.9.6-1.4z",
    mic: "M12 3.5a3 3 0 0 0-3 3v5a3 3 0 0 0 6 0v-5a3 3 0 0 0-3-3zM5.5 11a6.5 6.5 0 0 0 13 0M12 17.5v3",
    stop: "M7.5 7.5h9v9h-9z",
    copy: "M9 9h10.5v10.5H9zM5 15V4.5h10.5",
    edit: "M4.5 19.5h4l10-10-4-4-10 10z",
    pin: "M9 4h6l-1 6 3 3H7l3-3zM12 13v7",
    trash: "M5 7h14M10 7V4.5h4V7M7 7l.8 12.5h8.4L17 7",
    search: "M11 18a7 7 0 1 0 0-14 7 7 0 0 0 0 14zM20 20l-4-4",
    check: "M5 12.5l4.5 4.5L19 7.5",
    rewrite: "M11 3.5l1.7 4.3 4.3 1.7-4.3 1.7L11 15.5l-1.7-4.3L5 9.5l4.3-1.7zM17.5 14.5l.9 2.1 2.1.9-2.1.9-.9 2.1-.9-2.1-2.1-.9 2.1-.9z",
    close: "M6 6l12 12M18 6 6 18",
    plus: "M12 5v14M5 12h14",
    chevron: "M9 6l6 6-6 6",
    clock: "M12 7.5V12l3 2M20.5 12a8.5 8.5 0 1 1-17 0 8.5 8.5 0 0 1 17 0z",
  } as const;
  export type IconName = keyof typeof icons;
</script>

<script lang="ts">
  let { name, size = 16 }: { name: IconName; size?: number } = $props();
</script>

<svg class="icon" viewBox="0 0 24 24" width={size} height={size} aria-hidden="true"><path d={icons[name]} /></svg>
