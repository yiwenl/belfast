function Test2() {
  this.a = "a";
  let _b = "b";

  this.test = function() {
    console.log(_b);
    _b = "c";
    console.log(_b);
  };
}

export default Test2;
