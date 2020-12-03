const analysis = process.argv[2];

console.log(
  analysis
    .split(",")
    .map((line) => {
      const i = line.indexOf("=");
      const file = line.slice(0, i);
      const url = line.slice(i + 1);

      return `\`${file}\`: [VirusTotal analysis](${url})`;
    })
    .join("\n")
);
