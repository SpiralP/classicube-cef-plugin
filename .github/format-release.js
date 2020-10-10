const analysis = process.argv[2];
const version = process.env.GITHUB_REF.replace(/^refs\/tags\//, "").replace(
  /^v/,
  ""
);

const lines = [`Release version ${version}`, ""];

analysis
  .split(",")
  .map((line) => {
    const [file, url] = line.split("=");
    return `\`${file}\`: [VirusTotal analysis](${url})`;
  })
  .forEach((line) => {
    lines.push(line);
  });

console.log(lines.join("\n"));
