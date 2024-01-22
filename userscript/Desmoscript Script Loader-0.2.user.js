// ==UserScript==
// @name         Desmoscript Script Loader
// @namespace    http://tampermonkey.net/
// @version      0.2
// @description  Load scripts from http servers into desmos.
// @author       Radian628 + CATboardBETA
// @match        https://*.desmos.com/calculator
// @match        https://*.desmos.com/calculator/*
// @match        https://*.desmos.com/3d
// @match        https://*.desmos.com/3d/*
// @match        https://*.desmos.com/geometry
// @match        https://*.desmos.com/geometry/*
// @icon         data:image/gif;base64,R0lGODlhAQABAAAAACH5BAEKAAEALAAAAAABAAEAAAICTAEAOw==
// @grant        none
// ==/UserScript==

(function() {
    'use strict';

    const container = document.createElement("div");
    container.style.position = "absolute";
    container.style.bottom = "10px";
    container.style.right = "10px";
    container.style.zIndex = "999999999";
    container.style.display = "flex";
    container.style.background = "#ededed";
    container.style.flexDirection = "column";
    container.style.padding = "7px";
    container.className = "dcg-option dcg-btn-flat-gray";
    document.body.appendChild(container);

    const urlInput = document.createElement("textarea");
    urlInput.value = "http://localhost:8000/data";
    container.appendChild(urlInput);

    const getVersion = async () => await (await fetch(urlInput.value + "/version")).text();
    const recompile = async () => {
        recompileInput.innerText = "Fetching...";
        Calc.setState(await (await fetch(urlInput.value)).json());
        lastLoadedVersion = await getVersion();
        recompileInput.innerText = "Recompile";
    }

    const recompileInput = document.createElement("button");
    recompileInput.innerText = "Recompile";
    let lastLoadedVersion = "";
    recompileInput.onclick = recompile;
    container.appendChild(recompileInput);

    const autoRecompileLabel = document.createElement("label");
    const text = document.createElement("span");
    text.innerText = "Auto-recompile: ";
    autoRecompileLabel.appendChild(text);
    const autoRecompileInput = document.createElement("input");
    autoRecompileInput.type = "checkbox";
    autoRecompileInput.checked = true;
    autoRecompileLabel.appendChild(autoRecompileInput);
    container.appendChild(autoRecompileLabel);

    const recompileIfOutOfDate = async () => {
        if (autoRecompileInput.checked) {
           checkVersion: {
                let version = await getVersion();
                if (version == lastLoadedVersion) break checkVersion;
                await recompile();
            }
        }
        setTimeout(recompileIfOutOfDate, 200);
    }

    recompileIfOutOfDate();

    // Your code here...
})();