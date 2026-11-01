import {writeFile} from 'node:fs/promises';
const repository=process.env.GITHUB_REPOSITORY;
if(!repository||!/^[-\w.]+\/[-\w.]+$/.test(repository))throw Error('GITHUB_REPOSITORY must identify owner/repository.');
await writeFile('docs/demo-config.js',`window.EVERAFTER_REPOSITORY=${JSON.stringify('https://github.com/'+repository)};\n`);
