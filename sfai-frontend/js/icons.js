/* One monoline icon family, 24px canvas, 1.6px stroke. */
(function () {
  "use strict";

  var SFAI = (window.SFAI = window.SFAI || {});

  var symbols = {
    plus: '<path d="M12 5v14M5 12h14"/>',
    search: '<circle cx="11" cy="11" r="6.5"/><path d="m16 16 4 4"/>',
    tasks: '<path d="M9 6h11M9 12h11M9 18h7"/><path d="m3.5 6 1.5 1.5L7.5 5M3.5 12l1.5 1.5L7.5 11M3.5 18l1.5 1.5L7.5 17"/>',
    trajectory: '<path d="M3 12h4l2.5-6 4 12 2.5-6h5"/>',
    context: '<path d="m12 3 8 4.5-8 4.5-8-4.5L12 3Z"/><path d="m4 12 8 4.5 8-4.5M4 16.5 12 21l8-4.5"/>',
    capabilities: '<rect x="3.5" y="3.5" width="6" height="6" rx="1"/><rect x="14.5" y="3.5" width="6" height="6" rx="1"/><rect x="3.5" y="14.5" width="6" height="6" rx="1"/><rect x="14.5" y="14.5" width="6" height="6" rx="1"/><path d="M9.5 6.5h5M6.5 9.5v5M17.5 9.5v5M9.5 17.5h5"/>',
    settings: '<path d="M4 7h10M18 7h2M4 17h2M10 17h10M7 4v6M14 14v6"/><circle cx="16" cy="7" r="2"/><circle cx="8" cy="17" r="2"/>',
    close: '<path d="m6 6 12 12M18 6 6 18"/>',
    menu: '<path d="M4 7h16M4 12h16M4 17h16"/>',
    "chevron-down": '<path d="m6 9 6 6 6-6"/>',
    "chevron-right": '<path d="m9 6 6 6-6 6"/>',
    moon: '<path d="M20 14.5A8 8 0 0 1 9.5 4 8.2 8.2 0 1 0 20 14.5Z"/>',
    sun: '<circle cx="12" cy="12" r="4"/><path d="M12 2.5v2M12 19.5v2M4.6 4.6 6 6M18 18l1.4 1.4M2.5 12h2M19.5 12h2M4.6 19.4 6 18M18 6l1.4-1.4"/>',
    "panel-right": '<rect x="3.5" y="4.5" width="17" height="15" rx="2"/><path d="M14.5 4.5v15"/>',
    send: '<path d="M12 19V5M6.5 10.5 12 5l5.5 5.5"/>',
    pause: '<path d="M9 6v12M15 6v12"/>',
    play: '<path d="m8 5 11 7-11 7V5Z"/>',
    check: '<path d="m5 12.5 4.5 4.5L19 7"/>',
    circle: '<circle cx="12" cy="12" r="7.5"/>',
    alert: '<path d="M12 4 2.8 20h18.4L12 4Z"/><path d="M12 9v5M12 17.5v.1"/>',
    clock: '<circle cx="12" cy="12" r="8"/><path d="M12 7.5V12l3 2"/>',
    copy: '<rect x="8" y="8" width="11" height="11" rx="2"/><path d="M16 8V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2"/>',
    download: '<path d="M12 4v11M7.5 10.5 12 15l4.5-4.5M4.5 19.5h15"/>',
    lock: '<rect x="5" y="10" width="14" height="10" rx="2"/><path d="M8.5 10V7.5a3.5 3.5 0 0 1 7 0V10M12 14v2"/>',
    shield: '<path d="M12 3 5 6v5.5c0 4.2 2.8 7.6 7 9.5 4.2-1.9 7-5.3 7-9.5V6l-7-3Z"/><path d="m8.8 12 2.2 2.2 4.2-4.4"/>',
    file: '<path d="M6 3.5h7l5 5v12H6v-17Z"/><path d="M13 3.5v5h5M9 13h6M9 17h4"/>',
    external: '<path d="M14 4h6v6M20 4l-8 8"/><path d="M18 14v4a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h4"/>',
    info: '<circle cx="12" cy="12" r="8"/><path d="M12 11v5M12 8v.1"/>',
    refresh: '<path d="M20 7v5h-5"/><path d="M4 17v-5h5"/><path d="M6.1 8.2A7 7 0 0 1 19.6 10M4.4 14A7 7 0 0 0 17.9 15.8"/>',
    trash: '<path d="M4 7h16M9 7V4.5h6V7M6.5 7l1 13h9l1-13M10 11v5M14 11v5"/>',
    link: '<path d="m10 13.5 4-4"/><path d="M7.5 15.5 6 17a3.5 3.5 0 0 0 5 5l2.5-2.5M16.5 8.5 18 7a3.5 3.5 0 0 0-5-5L10.5 4.5"/>',
    "arrow-up-right": '<path d="M7 17 17 7M9 7h8v8"/>',
    "arrow-left": '<path d="M19 12H5M11 6l-6 6 6 6"/>',
    branch: '<circle cx="6" cy="5" r="2"/><circle cx="6" cy="19" r="2"/><circle cx="18" cy="12" r="2"/><path d="M6 7v10M8 5h2a4 4 0 0 1 4 4v.5M8 19h2a4 4 0 0 0 4-4v-.5"/>'
  };

  var sprite = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  sprite.setAttribute("aria-hidden", "true");
  sprite.setAttribute("focusable", "false");
  sprite.style.cssText = "position:absolute;width:0;height:0;overflow:hidden";
  sprite.innerHTML = Object.keys(symbols).map(function (name) {
    return '<symbol id="i-' + name + '" viewBox="0 0 24 24">' + symbols[name] + "</symbol>";
  }).join("");
  document.body.insertBefore(sprite, document.body.firstChild);

  function icon(name, className) {
    var safe = symbols[name] ? name : "circle";
    var extra = className ? " " + className : "";
    return '<svg class="icon' + extra + '" aria-hidden="true" focusable="false"><use href="#i-' + safe + '"></use></svg>';
  }

  function brandMark() {
    return '<svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">' +
      '<g fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round">' +
        '<path d="M6 6c4.5 0 7.5 2.5 12 6"/>' +
        '<path d="M6 18c4.5 0 7.5-2.5 12-6"/>' +
      "</g>" +
      '<circle cx="6" cy="6" r="2.1" fill="currentColor"/>' +
      '<circle cx="6" cy="18" r="2.1" fill="currentColor"/>' +
      '<circle cx="18" cy="12" r="2.5" fill="currentColor"/>' +
      "</svg>";
  }

  function hydrate(root) {
    (root || document).querySelectorAll("[data-icon-slot]").forEach(function (slot) {
      if (slot.dataset.hydrated === "true") return;
      var name = slot.getAttribute("data-icon-slot");
      slot.innerHTML = name === "brand" ? brandMark() : icon(name);
      slot.dataset.hydrated = "true";
    });
  }

  SFAI.icons = { icon: icon, brandMark: brandMark, hydrate: hydrate, symbols: symbols };
})();
