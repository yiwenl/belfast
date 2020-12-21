class Test1 {
  constructor() {
    this.a = "a";
    this._b = "b";
  }

  test() {
    console.log(this._b);
    this._b = "c";
    console.log(this._b);
  }
}

export default Test1;
