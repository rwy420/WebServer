const queryString = window.location.search;
const urlParams = new URLSearchParams(queryString);

const test = urlParams.get('test')
console.log(test);