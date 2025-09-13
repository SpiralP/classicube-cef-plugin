#!/usr/bin/env zx

/**
 * @typedef {Object} File
 * @property {"standard"|"minimal"|"client"} type
 * @property {string} name
 * @property {string} sha1
 */

/**
 * @typedef {Object} Version
 * @property {string} cef_version
 * @property {"stable"|"beta"} channel
 * @property {File[]} files
 */

/** @type {Record<string, { versions: Version[] }>} */
const versionsByOs = await (
  await fetch("https://cef-builds.spotifycdn.com/index.json")
).json();

const mainPlatform = "windows64";
// linux32, linux64, linuxarm, linuxarm64, macosarm64, macosx64, windows32, windows64, windowsarm64
const requiredPlatforms = ["windows64", "linux64", "macosx64"];

/** @param {string} chromeVersion */
function getBranch(chromeVersion) {
  return parseInt(chromeVersion.split(".")[2]);
}

function getLatestStableVersion() {
  const sortedVersions = versionsByOs[mainPlatform].versions.sort((a, b) => {
    return getBranch(b.chromium_version) - getBranch(a.chromium_version);
  });
  for (const version of sortedVersions) {
    const { cef_version, channel } = version;

    let ok = true;
    if (
      // skip beta versions
      version.channel !== "stable" ||
      version.files.some(({ name }) => name.includes("_beta"))
    ) {
      ok = false;
      console.warn("skipping beta version", cef_version);
    }
    for (const platform of requiredPlatforms) {
      if (
        !versionsByOs[platform].versions.find(
          (o) => o.cef_version === cef_version
        )
      ) {
        ok = false;
        console.warn("skipping version not found for required platform", {
          platform,
          cef_version,
        });
      }
    }
    if (ok) {
      return version;
    }
  }

  return null;
}

function main() {
  const latestVersion = getLatestStableVersion();
  if (!latestVersion) {
    console.warn("!latestVersion");
    process.exit(1);
    return;
  }
  console.log(latestVersion.cef_version);
  process.exit(0);
}

main();
