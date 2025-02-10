var kr = typeof globalThis < "u" ? globalThis : typeof window < "u" ? window : typeof global < "u" ? global : typeof self < "u" ? self : {};
function rn(t) {
  return t && t.__esModule && Object.prototype.hasOwnProperty.call(t, "default") ? t.default : t;
}
function tn(t) {
  if (t.__esModule) return t;
  var l = t.default;
  if (typeof l == "function") {
    var f = function u() {
      return this instanceof u ? Reflect.construct(l, arguments, this.constructor) : l.apply(this, arguments);
    };
    f.prototype = l.prototype;
  } else f = {};
  return Object.defineProperty(f, "__esModule", { value: !0 }), Object.keys(t).forEach(function(u) {
    var S = Object.getOwnPropertyDescriptor(t, u);
    Object.defineProperty(f, u, S.get ? S : {
      enumerable: !0,
      get: function() {
        return t[u];
      }
    });
  }), f;
}
var Re = { exports: {} }, Be = {}, Le, jr;
function be() {
  return jr || (jr = 1, Le = TypeError), Le;
}
const nn = {}, an = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  default: nn
}, Symbol.toStringTag, { value: "Module" })), on = /* @__PURE__ */ tn(an);
var Ue, zr;
function qe() {
  if (zr) return Ue;
  zr = 1;
  var t = typeof Map == "function" && Map.prototype, l = Object.getOwnPropertyDescriptor && t ? Object.getOwnPropertyDescriptor(Map.prototype, "size") : null, f = t && l && typeof l.get == "function" ? l.get : null, u = t && Map.prototype.forEach, S = typeof Set == "function" && Set.prototype, w = Object.getOwnPropertyDescriptor && S ? Object.getOwnPropertyDescriptor(Set.prototype, "size") : null, P = S && w && typeof w.get == "function" ? w.get : null, p = S && Set.prototype.forEach, s = typeof WeakMap == "function" && WeakMap.prototype, v = s ? WeakMap.prototype.has : null, d = typeof WeakSet == "function" && WeakSet.prototype, h = d ? WeakSet.prototype.has : null, g = typeof WeakRef == "function" && WeakRef.prototype, i = g ? WeakRef.prototype.deref : null, c = Boolean.prototype.valueOf, b = Object.prototype.toString, n = Function.prototype.toString, a = String.prototype.match, o = String.prototype.slice, m = String.prototype.replace, y = String.prototype.toUpperCase, A = String.prototype.toLowerCase, O = RegExp.prototype.test, E = Array.prototype.concat, q = Array.prototype.join, C = Array.prototype.slice, _ = Math.floor, M = typeof BigInt == "function" ? BigInt.prototype.valueOf : null, R = Object.getOwnPropertySymbols, G = typeof Symbol == "function" && typeof Symbol.iterator == "symbol" ? Symbol.prototype.toString : null, L = typeof Symbol == "function" && typeof Symbol.iterator == "object", z = typeof Symbol == "function" && Symbol.toStringTag && (typeof Symbol.toStringTag === L || !0) ? Symbol.toStringTag : null, ee = Object.prototype.propertyIsEnumerable, re = (typeof Reflect == "function" ? Reflect.getPrototypeOf : Object.getPrototypeOf) || ([].__proto__ === Array.prototype ? function(e) {
    return e.__proto__;
  } : null);
  function I(e, r) {
    if (e === 1 / 0 || e === -1 / 0 || e !== e || e && e > -1e3 && e < 1e3 || O.call(/e/, r))
      return r;
    var D = /[0-9](?=(?:[0-9]{3})+(?![0-9]))/g;
    if (typeof e == "number") {
      var $ = e < 0 ? -_(-e) : _(e);
      if ($ !== e) {
        var N = String($), x = o.call(r, N.length + 1);
        return m.call(N, D, "$&_") + "." + m.call(m.call(x, /([0-9]{3})/g, "$&_"), /_$/, "");
      }
    }
    return m.call(r, D, "$&_");
  }
  var K = on, te = K.custom, se = H(te) ? te : null, ae = {
    __proto__: null,
    double: '"',
    single: "'"
  }, oe = {
    __proto__: null,
    double: /(["\\])/g,
    single: /(['\\])/g
  };
  Ue = function e(r, D, $, N) {
    var x = D || {};
    if (j(x, "quoteStyle") && !j(ae, x.quoteStyle))
      throw new TypeError('option "quoteStyle" must be "single" or "double"');
    if (j(x, "maxStringLength") && (typeof x.maxStringLength == "number" ? x.maxStringLength < 0 && x.maxStringLength !== 1 / 0 : x.maxStringLength !== null))
      throw new TypeError('option "maxStringLength", if provided, must be a positive integer, Infinity, or `null`');
    var ce = j(x, "customInspect") ? x.customInspect : !0;
    if (typeof ce != "boolean" && ce !== "symbol")
      throw new TypeError("option \"customInspect\", if provided, must be `true`, `false`, or `'symbol'`");
    if (j(x, "indent") && x.indent !== null && x.indent !== "	" && !(parseInt(x.indent, 10) === x.indent && x.indent > 0))
      throw new TypeError('option "indent" must be "\\t", an integer > 0, or `null`');
    if (j(x, "numericSeparator") && typeof x.numericSeparator != "boolean")
      throw new TypeError('option "numericSeparator", if provided, must be `true` or `false`');
    var ye = x.numericSeparator;
    if (typeof r > "u")
      return "undefined";
    if (r === null)
      return "null";
    if (typeof r == "boolean")
      return r ? "true" : "false";
    if (typeof r == "string")
      return Mr(r, x);
    if (typeof r == "number") {
      if (r === 0)
        return 1 / 0 / r > 0 ? "0" : "-0";
      var Q = String(r);
      return ye ? I(r, Q) : Q;
    }
    if (typeof r == "bigint") {
      var pe = String(r) + "n";
      return ye ? I(r, pe) : pe;
    }
    var Fe = typeof x.depth > "u" ? 5 : x.depth;
    if (typeof $ > "u" && ($ = 0), $ >= Fe && Fe > 0 && typeof r == "object")
      return ue(r) ? "[Array]" : "[Object]";
    var me = Yt(x, $);
    if (typeof N > "u")
      N = [];
    else if (le(N, r) >= 0)
      return "[Circular]";
    function Z(Se, Pe, en) {
      if (Pe && (N = C.call(N), N.push(Pe)), en) {
        var Hr = {
          depth: x.depth
        };
        return j(x, "quoteStyle") && (Hr.quoteStyle = x.quoteStyle), e(Se, Hr, $ + 1, N);
      }
      return e(Se, x, $ + 1, N);
    }
    if (typeof r == "function" && !W(r)) {
      var $r = de(r), Nr = Oe(r, Z);
      return "[Function" + ($r ? ": " + $r : " (anonymous)") + "]" + (Nr.length > 0 ? " { " + q.call(Nr, ", ") + " }" : "");
    }
    if (H(r)) {
      var Br = L ? m.call(String(r), /^(Symbol\(.*\))_[^)]*$/, "$1") : G.call(r);
      return typeof r == "object" && !L ? we(Br) : Br;
    }
    if (Vt(r)) {
      for (var Ae = "<" + A.call(String(r.nodeName)), De = r.attributes || [], Ee = 0; Ee < De.length; Ee++)
        Ae += " " + De[Ee].name + "=" + ie(ne(De[Ee].value), "double", x);
      return Ae += ">", r.childNodes && r.childNodes.length && (Ae += "..."), Ae += "</" + A.call(String(r.nodeName)) + ">", Ae;
    }
    if (ue(r)) {
      if (r.length === 0)
        return "[]";
      var Ce = Oe(r, Z);
      return me && !Xt(Ce) ? "[" + Ie(Ce, me) + "]" : "[ " + q.call(Ce, ", ") + " ]";
    }
    if (F(r)) {
      var Me = Oe(r, Z);
      return !("cause" in Error.prototype) && "cause" in r && !ee.call(r, "cause") ? "{ [" + String(r) + "] " + q.call(E.call("[cause]: " + Z(r.cause), Me), ", ") + " }" : Me.length === 0 ? "[" + String(r) + "]" : "{ [" + String(r) + "] " + q.call(Me, ", ") + " }";
    }
    if (typeof r == "object" && ce) {
      if (se && typeof r[se] == "function" && K)
        return K(r, { depth: Fe - $ });
      if (ce !== "symbol" && typeof r.inspect == "function")
        return r.inspect();
    }
    if (Y(r)) {
      var Lr = [];
      return u && u.call(r, function(Se, Pe) {
        Lr.push(Z(Pe, r, !0) + " => " + Z(Se, r));
      }), Tr("Map", f.call(r), Lr, me);
    }
    if (ge(r)) {
      var Ur = [];
      return p && p.call(r, function(Se) {
        Ur.push(Z(Se, r));
      }), Tr("Set", P.call(r), Ur, me);
    }
    if (ve(r))
      return xe("WeakMap");
    if (Qt(r))
      return xe("WeakSet");
    if (he(r))
      return xe("WeakRef");
    if (T(r))
      return we(Z(Number(r)));
    if (J(r))
      return we(Z(M.call(r)));
    if (U(r))
      return we(c.call(r));
    if (B(r))
      return we(Z(String(r)));
    if (typeof window < "u" && r === window)
      return "{ [object Window] }";
    if (typeof globalThis < "u" && r === globalThis || typeof kr < "u" && r === kr)
      return "{ [object globalThis] }";
    if (!fe(r) && !W(r)) {
      var Te = Oe(r, Z), Wr = re ? re(r) === Object.prototype : r instanceof Object || r.constructor === Object, $e = r instanceof Object ? "" : "null prototype", Gr = !Wr && z && Object(r) === r && z in r ? o.call(X(r), 8, -1) : $e ? "Object" : "", Zt = Wr || typeof r.constructor != "function" ? "" : r.constructor.name ? r.constructor.name + " " : "", Ne = Zt + (Gr || $e ? "[" + q.call(E.call([], Gr || [], $e || []), ": ") + "] " : "");
      return Te.length === 0 ? Ne + "{}" : me ? Ne + "{" + Ie(Te, me) + "}" : Ne + "{ " + q.call(Te, ", ") + " }";
    }
    return String(r);
  };
  function ie(e, r, D) {
    var $ = D.quoteStyle || r, N = ae[$];
    return N + e + N;
  }
  function ne(e) {
    return m.call(String(e), /"/g, "&quot;");
  }
  function V(e) {
    return !z || !(typeof e == "object" && (z in e || typeof e[z] < "u"));
  }
  function ue(e) {
    return X(e) === "[object Array]" && V(e);
  }
  function fe(e) {
    return X(e) === "[object Date]" && V(e);
  }
  function W(e) {
    return X(e) === "[object RegExp]" && V(e);
  }
  function F(e) {
    return X(e) === "[object Error]" && V(e);
  }
  function B(e) {
    return X(e) === "[object String]" && V(e);
  }
  function T(e) {
    return X(e) === "[object Number]" && V(e);
  }
  function U(e) {
    return X(e) === "[object Boolean]" && V(e);
  }
  function H(e) {
    if (L)
      return e && typeof e == "object" && e instanceof Symbol;
    if (typeof e == "symbol")
      return !0;
    if (!e || typeof e != "object" || !G)
      return !1;
    try {
      return G.call(e), !0;
    } catch {
    }
    return !1;
  }
  function J(e) {
    if (!e || typeof e != "object" || !M)
      return !1;
    try {
      return M.call(e), !0;
    } catch {
    }
    return !1;
  }
  var k = Object.prototype.hasOwnProperty || function(e) {
    return e in this;
  };
  function j(e, r) {
    return k.call(e, r);
  }
  function X(e) {
    return b.call(e);
  }
  function de(e) {
    if (e.name)
      return e.name;
    var r = a.call(n.call(e), /^function\s*([\w$]+)/);
    return r ? r[1] : null;
  }
  function le(e, r) {
    if (e.indexOf)
      return e.indexOf(r);
    for (var D = 0, $ = e.length; D < $; D++)
      if (e[D] === r)
        return D;
    return -1;
  }
  function Y(e) {
    if (!f || !e || typeof e != "object")
      return !1;
    try {
      f.call(e);
      try {
        P.call(e);
      } catch {
        return !0;
      }
      return e instanceof Map;
    } catch {
    }
    return !1;
  }
  function ve(e) {
    if (!v || !e || typeof e != "object")
      return !1;
    try {
      v.call(e, v);
      try {
        h.call(e, h);
      } catch {
        return !0;
      }
      return e instanceof WeakMap;
    } catch {
    }
    return !1;
  }
  function he(e) {
    if (!i || !e || typeof e != "object")
      return !1;
    try {
      return i.call(e), !0;
    } catch {
    }
    return !1;
  }
  function ge(e) {
    if (!P || !e || typeof e != "object")
      return !1;
    try {
      P.call(e);
      try {
        f.call(e);
      } catch {
        return !0;
      }
      return e instanceof Set;
    } catch {
    }
    return !1;
  }
  function Qt(e) {
    if (!h || !e || typeof e != "object")
      return !1;
    try {
      h.call(e, h);
      try {
        v.call(e, v);
      } catch {
        return !0;
      }
      return e instanceof WeakSet;
    } catch {
    }
    return !1;
  }
  function Vt(e) {
    return !e || typeof e != "object" ? !1 : typeof HTMLElement < "u" && e instanceof HTMLElement ? !0 : typeof e.nodeName == "string" && typeof e.getAttribute == "function";
  }
  function Mr(e, r) {
    if (e.length > r.maxStringLength) {
      var D = e.length - r.maxStringLength, $ = "... " + D + " more character" + (D > 1 ? "s" : "");
      return Mr(o.call(e, 0, r.maxStringLength), r) + $;
    }
    var N = oe[r.quoteStyle || "single"];
    N.lastIndex = 0;
    var x = m.call(m.call(e, N, "\\$1"), /[\x00-\x1f]/g, Jt);
    return ie(x, "single", r);
  }
  function Jt(e) {
    var r = e.charCodeAt(0), D = {
      8: "b",
      9: "t",
      10: "n",
      12: "f",
      13: "r"
    }[r];
    return D ? "\\" + D : "\\x" + (r < 16 ? "0" : "") + y.call(r.toString(16));
  }
  function we(e) {
    return "Object(" + e + ")";
  }
  function xe(e) {
    return e + " { ? }";
  }
  function Tr(e, r, D, $) {
    var N = $ ? Ie(D, $) : q.call(D, ", ");
    return e + " (" + r + ") {" + N + "}";
  }
  function Xt(e) {
    for (var r = 0; r < e.length; r++)
      if (le(e[r], `
`) >= 0)
        return !1;
    return !0;
  }
  function Yt(e, r) {
    var D;
    if (e.indent === "	")
      D = "	";
    else if (typeof e.indent == "number" && e.indent > 0)
      D = q.call(Array(e.indent + 1), " ");
    else
      return null;
    return {
      base: D,
      prev: q.call(Array(r + 1), D)
    };
  }
  function Ie(e, r) {
    if (e.length === 0)
      return "";
    var D = `
` + r.prev + r.base;
    return D + q.call(e, "," + D) + `
` + r.prev;
  }
  function Oe(e, r) {
    var D = ue(e), $ = [];
    if (D) {
      $.length = e.length;
      for (var N = 0; N < e.length; N++)
        $[N] = j(e, N) ? r(e[N], e) : "";
    }
    var x = typeof R == "function" ? R(e) : [], ce;
    if (L) {
      ce = {};
      for (var ye = 0; ye < x.length; ye++)
        ce["$" + x[ye]] = x[ye];
    }
    for (var Q in e)
      j(e, Q) && (D && String(Number(Q)) === Q && Q < e.length || L && ce["$" + Q] instanceof Symbol || (O.call(/[^\w$]/, Q) ? $.push(r(Q, e) + ": " + r(e[Q], e)) : $.push(Q + ": " + r(e[Q], e))));
    if (typeof R == "function")
      for (var pe = 0; pe < x.length; pe++)
        ee.call(e, x[pe]) && $.push("[" + r(x[pe]) + "]: " + r(e[x[pe]], e));
    return $;
  }
  return Ue;
}
var We, Kr;
function un() {
  if (Kr) return We;
  Kr = 1;
  var t = /* @__PURE__ */ qe(), l = /* @__PURE__ */ be(), f = function(p, s, v) {
    for (var d = p, h; (h = d.next) != null; d = h)
      if (h.key === s)
        return d.next = h.next, v || (h.next = /** @type {NonNullable<typeof list.next>} */
        p.next, p.next = h), h;
  }, u = function(p, s) {
    if (p) {
      var v = f(p, s);
      return v && v.value;
    }
  }, S = function(p, s, v) {
    var d = f(p, s);
    d ? d.value = v : p.next = /** @type {import('./list.d.ts').ListNode<typeof value, typeof key>} */
    {
      // eslint-disable-line no-param-reassign, no-extra-parens
      key: s,
      next: p.next,
      value: v
    };
  }, w = function(p, s) {
    return p ? !!f(p, s) : !1;
  }, P = function(p, s) {
    if (p)
      return f(p, s, !0);
  };
  return We = function() {
    var s, v = {
      assert: function(d) {
        if (!v.has(d))
          throw new l("Side channel does not contain " + t(d));
      },
      delete: function(d) {
        var h = s && s.next, g = P(s, d);
        return g && h && h === g && (s = void 0), !!g;
      },
      get: function(d) {
        return u(s, d);
      },
      has: function(d) {
        return w(s, d);
      },
      set: function(d, h) {
        s || (s = {
          next: void 0
        }), S(
          /** @type {NonNullable<typeof $o>} */
          s,
          d,
          h
        );
      }
    };
    return v;
  }, We;
}
var Ge, Qr;
function Lt() {
  return Qr || (Qr = 1, Ge = Object), Ge;
}
var He, Vr;
function fn() {
  return Vr || (Vr = 1, He = Error), He;
}
var ke, Jr;
function ln() {
  return Jr || (Jr = 1, ke = EvalError), ke;
}
var je, Xr;
function cn() {
  return Xr || (Xr = 1, je = RangeError), je;
}
var ze, Yr;
function pn() {
  return Yr || (Yr = 1, ze = ReferenceError), ze;
}
var Ke, Zr;
function sn() {
  return Zr || (Zr = 1, Ke = SyntaxError), Ke;
}
var Qe, et;
function yn() {
  return et || (et = 1, Qe = URIError), Qe;
}
var Ve, rt;
function dn() {
  return rt || (rt = 1, Ve = Math.abs), Ve;
}
var Je, tt;
function vn() {
  return tt || (tt = 1, Je = Math.floor), Je;
}
var Xe, nt;
function hn() {
  return nt || (nt = 1, Xe = Math.max), Xe;
}
var Ye, at;
function gn() {
  return at || (at = 1, Ye = Math.min), Ye;
}
var Ze, ot;
function mn() {
  return ot || (ot = 1, Ze = Math.pow), Ze;
}
var er, it;
function Sn() {
  return it || (it = 1, er = Math.round), er;
}
var rr, ut;
function bn() {
  return ut || (ut = 1, rr = Number.isNaN || function(l) {
    return l !== l;
  }), rr;
}
var tr, ft;
function wn() {
  if (ft) return tr;
  ft = 1;
  var t = /* @__PURE__ */ bn();
  return tr = function(f) {
    return t(f) || f === 0 ? f : f < 0 ? -1 : 1;
  }, tr;
}
var nr, lt;
function An() {
  return lt || (lt = 1, nr = Object.getOwnPropertyDescriptor), nr;
}
var ar, ct;
function Ut() {
  if (ct) return ar;
  ct = 1;
  var t = /* @__PURE__ */ An();
  if (t)
    try {
      t([], "length");
    } catch {
      t = null;
    }
  return ar = t, ar;
}
var or, pt;
function On() {
  if (pt) return or;
  pt = 1;
  var t = Object.defineProperty || !1;
  if (t)
    try {
      t({}, "a", { value: 1 });
    } catch {
      t = !1;
    }
  return or = t, or;
}
var ir, st;
function En() {
  return st || (st = 1, ir = function() {
    if (typeof Symbol != "function" || typeof Object.getOwnPropertySymbols != "function")
      return !1;
    if (typeof Symbol.iterator == "symbol")
      return !0;
    var l = {}, f = Symbol("test"), u = Object(f);
    if (typeof f == "string" || Object.prototype.toString.call(f) !== "[object Symbol]" || Object.prototype.toString.call(u) !== "[object Symbol]")
      return !1;
    var S = 42;
    l[f] = S;
    for (var w in l)
      return !1;
    if (typeof Object.keys == "function" && Object.keys(l).length !== 0 || typeof Object.getOwnPropertyNames == "function" && Object.getOwnPropertyNames(l).length !== 0)
      return !1;
    var P = Object.getOwnPropertySymbols(l);
    if (P.length !== 1 || P[0] !== f || !Object.prototype.propertyIsEnumerable.call(l, f))
      return !1;
    if (typeof Object.getOwnPropertyDescriptor == "function") {
      var p = (
        /** @type {PropertyDescriptor} */
        Object.getOwnPropertyDescriptor(l, f)
      );
      if (p.value !== S || p.enumerable !== !0)
        return !1;
    }
    return !0;
  }), ir;
}
var ur, yt;
function Pn() {
  if (yt) return ur;
  yt = 1;
  var t = typeof Symbol < "u" && Symbol, l = En();
  return ur = function() {
    return typeof t != "function" || typeof Symbol != "function" || typeof t("foo") != "symbol" || typeof Symbol("bar") != "symbol" ? !1 : l();
  }, ur;
}
var fr, dt;
function Wt() {
  return dt || (dt = 1, fr = typeof Reflect < "u" && Reflect.getPrototypeOf || null), fr;
}
var lr, vt;
function Gt() {
  if (vt) return lr;
  vt = 1;
  var t = /* @__PURE__ */ Lt();
  return lr = t.getPrototypeOf || null, lr;
}
var cr, ht;
function Rn() {
  if (ht) return cr;
  ht = 1;
  var t = "Function.prototype.bind called on incompatible ", l = Object.prototype.toString, f = Math.max, u = "[object Function]", S = function(s, v) {
    for (var d = [], h = 0; h < s.length; h += 1)
      d[h] = s[h];
    for (var g = 0; g < v.length; g += 1)
      d[g + s.length] = v[g];
    return d;
  }, w = function(s, v) {
    for (var d = [], h = v, g = 0; h < s.length; h += 1, g += 1)
      d[g] = s[h];
    return d;
  }, P = function(p, s) {
    for (var v = "", d = 0; d < p.length; d += 1)
      v += p[d], d + 1 < p.length && (v += s);
    return v;
  };
  return cr = function(s) {
    var v = this;
    if (typeof v != "function" || l.apply(v) !== u)
      throw new TypeError(t + v);
    for (var d = w(arguments, 1), h, g = function() {
      if (this instanceof h) {
        var a = v.apply(
          this,
          S(d, arguments)
        );
        return Object(a) === a ? a : this;
      }
      return v.apply(
        s,
        S(d, arguments)
      );
    }, i = f(0, v.length - d.length), c = [], b = 0; b < i; b++)
      c[b] = "$" + b;
    if (h = Function("binder", "return function (" + P(c, ",") + "){ return binder.apply(this,arguments); }")(g), v.prototype) {
      var n = function() {
      };
      n.prototype = v.prototype, h.prototype = new n(), n.prototype = null;
    }
    return h;
  }, cr;
}
var pr, gt;
function _e() {
  if (gt) return pr;
  gt = 1;
  var t = Rn();
  return pr = Function.prototype.bind || t, pr;
}
var sr, mt;
function Fr() {
  return mt || (mt = 1, sr = Function.prototype.call), sr;
}
var yr, St;
function Ht() {
  return St || (St = 1, yr = Function.prototype.apply), yr;
}
var dr, bt;
function qn() {
  return bt || (bt = 1, dr = typeof Reflect < "u" && Reflect && Reflect.apply), dr;
}
var vr, wt;
function _n() {
  if (wt) return vr;
  wt = 1;
  var t = _e(), l = Ht(), f = Fr(), u = qn();
  return vr = u || t.call(f, l), vr;
}
var hr, At;
function kt() {
  if (At) return hr;
  At = 1;
  var t = _e(), l = /* @__PURE__ */ be(), f = Fr(), u = _n();
  return hr = function(w) {
    if (w.length < 1 || typeof w[0] != "function")
      throw new l("a function is required");
    return u(t, f, w);
  }, hr;
}
var gr, Ot;
function xn() {
  if (Ot) return gr;
  Ot = 1;
  var t = kt(), l = /* @__PURE__ */ Ut(), f;
  try {
    f = /** @type {{ __proto__?: typeof Array.prototype }} */
    [].__proto__ === Array.prototype;
  } catch (P) {
    if (!P || typeof P != "object" || !("code" in P) || P.code !== "ERR_PROTO_ACCESS")
      throw P;
  }
  var u = !!f && l && l(
    Object.prototype,
    /** @type {keyof typeof Object.prototype} */
    "__proto__"
  ), S = Object, w = S.getPrototypeOf;
  return gr = u && typeof u.get == "function" ? t([u.get]) : typeof w == "function" ? (
    /** @type {import('./get')} */
    function(p) {
      return w(p == null ? p : S(p));
    }
  ) : !1, gr;
}
var mr, Et;
function In() {
  if (Et) return mr;
  Et = 1;
  var t = Wt(), l = Gt(), f = /* @__PURE__ */ xn();
  return mr = t ? function(S) {
    return t(S);
  } : l ? function(S) {
    if (!S || typeof S != "object" && typeof S != "function")
      throw new TypeError("getProto: not an object");
    return l(S);
  } : f ? function(S) {
    return f(S);
  } : null, mr;
}
var Sr, Pt;
function Fn() {
  if (Pt) return Sr;
  Pt = 1;
  var t = Function.prototype.call, l = Object.prototype.hasOwnProperty, f = _e();
  return Sr = f.call(t, l), Sr;
}
var br, Rt;
function Dr() {
  if (Rt) return br;
  Rt = 1;
  var t, l = /* @__PURE__ */ Lt(), f = /* @__PURE__ */ fn(), u = /* @__PURE__ */ ln(), S = /* @__PURE__ */ cn(), w = /* @__PURE__ */ pn(), P = /* @__PURE__ */ sn(), p = /* @__PURE__ */ be(), s = /* @__PURE__ */ yn(), v = /* @__PURE__ */ dn(), d = /* @__PURE__ */ vn(), h = /* @__PURE__ */ hn(), g = /* @__PURE__ */ gn(), i = /* @__PURE__ */ mn(), c = /* @__PURE__ */ Sn(), b = /* @__PURE__ */ wn(), n = Function, a = function(W) {
    try {
      return n('"use strict"; return (' + W + ").constructor;")();
    } catch {
    }
  }, o = /* @__PURE__ */ Ut(), m = /* @__PURE__ */ On(), y = function() {
    throw new p();
  }, A = o ? function() {
    try {
      return arguments.callee, y;
    } catch {
      try {
        return o(arguments, "callee").get;
      } catch {
        return y;
      }
    }
  }() : y, O = Pn()(), E = In(), q = Gt(), C = Wt(), _ = Ht(), M = Fr(), R = {}, G = typeof Uint8Array > "u" || !E ? t : E(Uint8Array), L = {
    __proto__: null,
    "%AggregateError%": typeof AggregateError > "u" ? t : AggregateError,
    "%Array%": Array,
    "%ArrayBuffer%": typeof ArrayBuffer > "u" ? t : ArrayBuffer,
    "%ArrayIteratorPrototype%": O && E ? E([][Symbol.iterator]()) : t,
    "%AsyncFromSyncIteratorPrototype%": t,
    "%AsyncFunction%": R,
    "%AsyncGenerator%": R,
    "%AsyncGeneratorFunction%": R,
    "%AsyncIteratorPrototype%": R,
    "%Atomics%": typeof Atomics > "u" ? t : Atomics,
    "%BigInt%": typeof BigInt > "u" ? t : BigInt,
    "%BigInt64Array%": typeof BigInt64Array > "u" ? t : BigInt64Array,
    "%BigUint64Array%": typeof BigUint64Array > "u" ? t : BigUint64Array,
    "%Boolean%": Boolean,
    "%DataView%": typeof DataView > "u" ? t : DataView,
    "%Date%": Date,
    "%decodeURI%": decodeURI,
    "%decodeURIComponent%": decodeURIComponent,
    "%encodeURI%": encodeURI,
    "%encodeURIComponent%": encodeURIComponent,
    "%Error%": f,
    "%eval%": eval,
    // eslint-disable-line no-eval
    "%EvalError%": u,
    "%Float32Array%": typeof Float32Array > "u" ? t : Float32Array,
    "%Float64Array%": typeof Float64Array > "u" ? t : Float64Array,
    "%FinalizationRegistry%": typeof FinalizationRegistry > "u" ? t : FinalizationRegistry,
    "%Function%": n,
    "%GeneratorFunction%": R,
    "%Int8Array%": typeof Int8Array > "u" ? t : Int8Array,
    "%Int16Array%": typeof Int16Array > "u" ? t : Int16Array,
    "%Int32Array%": typeof Int32Array > "u" ? t : Int32Array,
    "%isFinite%": isFinite,
    "%isNaN%": isNaN,
    "%IteratorPrototype%": O && E ? E(E([][Symbol.iterator]())) : t,
    "%JSON%": typeof JSON == "object" ? JSON : t,
    "%Map%": typeof Map > "u" ? t : Map,
    "%MapIteratorPrototype%": typeof Map > "u" || !O || !E ? t : E((/* @__PURE__ */ new Map())[Symbol.iterator]()),
    "%Math%": Math,
    "%Number%": Number,
    "%Object%": l,
    "%Object.getOwnPropertyDescriptor%": o,
    "%parseFloat%": parseFloat,
    "%parseInt%": parseInt,
    "%Promise%": typeof Promise > "u" ? t : Promise,
    "%Proxy%": typeof Proxy > "u" ? t : Proxy,
    "%RangeError%": S,
    "%ReferenceError%": w,
    "%Reflect%": typeof Reflect > "u" ? t : Reflect,
    "%RegExp%": RegExp,
    "%Set%": typeof Set > "u" ? t : Set,
    "%SetIteratorPrototype%": typeof Set > "u" || !O || !E ? t : E((/* @__PURE__ */ new Set())[Symbol.iterator]()),
    "%SharedArrayBuffer%": typeof SharedArrayBuffer > "u" ? t : SharedArrayBuffer,
    "%String%": String,
    "%StringIteratorPrototype%": O && E ? E(""[Symbol.iterator]()) : t,
    "%Symbol%": O ? Symbol : t,
    "%SyntaxError%": P,
    "%ThrowTypeError%": A,
    "%TypedArray%": G,
    "%TypeError%": p,
    "%Uint8Array%": typeof Uint8Array > "u" ? t : Uint8Array,
    "%Uint8ClampedArray%": typeof Uint8ClampedArray > "u" ? t : Uint8ClampedArray,
    "%Uint16Array%": typeof Uint16Array > "u" ? t : Uint16Array,
    "%Uint32Array%": typeof Uint32Array > "u" ? t : Uint32Array,
    "%URIError%": s,
    "%WeakMap%": typeof WeakMap > "u" ? t : WeakMap,
    "%WeakRef%": typeof WeakRef > "u" ? t : WeakRef,
    "%WeakSet%": typeof WeakSet > "u" ? t : WeakSet,
    "%Function.prototype.call%": M,
    "%Function.prototype.apply%": _,
    "%Object.defineProperty%": m,
    "%Object.getPrototypeOf%": q,
    "%Math.abs%": v,
    "%Math.floor%": d,
    "%Math.max%": h,
    "%Math.min%": g,
    "%Math.pow%": i,
    "%Math.round%": c,
    "%Math.sign%": b,
    "%Reflect.getPrototypeOf%": C
  };
  if (E)
    try {
      null.error;
    } catch (W) {
      var z = E(E(W));
      L["%Error.prototype%"] = z;
    }
  var ee = function W(F) {
    var B;
    if (F === "%AsyncFunction%")
      B = a("async function () {}");
    else if (F === "%GeneratorFunction%")
      B = a("function* () {}");
    else if (F === "%AsyncGeneratorFunction%")
      B = a("async function* () {}");
    else if (F === "%AsyncGenerator%") {
      var T = W("%AsyncGeneratorFunction%");
      T && (B = T.prototype);
    } else if (F === "%AsyncIteratorPrototype%") {
      var U = W("%AsyncGenerator%");
      U && E && (B = E(U.prototype));
    }
    return L[F] = B, B;
  }, re = {
    __proto__: null,
    "%ArrayBufferPrototype%": ["ArrayBuffer", "prototype"],
    "%ArrayPrototype%": ["Array", "prototype"],
    "%ArrayProto_entries%": ["Array", "prototype", "entries"],
    "%ArrayProto_forEach%": ["Array", "prototype", "forEach"],
    "%ArrayProto_keys%": ["Array", "prototype", "keys"],
    "%ArrayProto_values%": ["Array", "prototype", "values"],
    "%AsyncFunctionPrototype%": ["AsyncFunction", "prototype"],
    "%AsyncGenerator%": ["AsyncGeneratorFunction", "prototype"],
    "%AsyncGeneratorPrototype%": ["AsyncGeneratorFunction", "prototype", "prototype"],
    "%BooleanPrototype%": ["Boolean", "prototype"],
    "%DataViewPrototype%": ["DataView", "prototype"],
    "%DatePrototype%": ["Date", "prototype"],
    "%ErrorPrototype%": ["Error", "prototype"],
    "%EvalErrorPrototype%": ["EvalError", "prototype"],
    "%Float32ArrayPrototype%": ["Float32Array", "prototype"],
    "%Float64ArrayPrototype%": ["Float64Array", "prototype"],
    "%FunctionPrototype%": ["Function", "prototype"],
    "%Generator%": ["GeneratorFunction", "prototype"],
    "%GeneratorPrototype%": ["GeneratorFunction", "prototype", "prototype"],
    "%Int8ArrayPrototype%": ["Int8Array", "prototype"],
    "%Int16ArrayPrototype%": ["Int16Array", "prototype"],
    "%Int32ArrayPrototype%": ["Int32Array", "prototype"],
    "%JSONParse%": ["JSON", "parse"],
    "%JSONStringify%": ["JSON", "stringify"],
    "%MapPrototype%": ["Map", "prototype"],
    "%NumberPrototype%": ["Number", "prototype"],
    "%ObjectPrototype%": ["Object", "prototype"],
    "%ObjProto_toString%": ["Object", "prototype", "toString"],
    "%ObjProto_valueOf%": ["Object", "prototype", "valueOf"],
    "%PromisePrototype%": ["Promise", "prototype"],
    "%PromiseProto_then%": ["Promise", "prototype", "then"],
    "%Promise_all%": ["Promise", "all"],
    "%Promise_reject%": ["Promise", "reject"],
    "%Promise_resolve%": ["Promise", "resolve"],
    "%RangeErrorPrototype%": ["RangeError", "prototype"],
    "%ReferenceErrorPrototype%": ["ReferenceError", "prototype"],
    "%RegExpPrototype%": ["RegExp", "prototype"],
    "%SetPrototype%": ["Set", "prototype"],
    "%SharedArrayBufferPrototype%": ["SharedArrayBuffer", "prototype"],
    "%StringPrototype%": ["String", "prototype"],
    "%SymbolPrototype%": ["Symbol", "prototype"],
    "%SyntaxErrorPrototype%": ["SyntaxError", "prototype"],
    "%TypedArrayPrototype%": ["TypedArray", "prototype"],
    "%TypeErrorPrototype%": ["TypeError", "prototype"],
    "%Uint8ArrayPrototype%": ["Uint8Array", "prototype"],
    "%Uint8ClampedArrayPrototype%": ["Uint8ClampedArray", "prototype"],
    "%Uint16ArrayPrototype%": ["Uint16Array", "prototype"],
    "%Uint32ArrayPrototype%": ["Uint32Array", "prototype"],
    "%URIErrorPrototype%": ["URIError", "prototype"],
    "%WeakMapPrototype%": ["WeakMap", "prototype"],
    "%WeakSetPrototype%": ["WeakSet", "prototype"]
  }, I = _e(), K = /* @__PURE__ */ Fn(), te = I.call(M, Array.prototype.concat), se = I.call(_, Array.prototype.splice), ae = I.call(M, String.prototype.replace), oe = I.call(M, String.prototype.slice), ie = I.call(M, RegExp.prototype.exec), ne = /[^%.[\]]+|\[(?:(-?\d+(?:\.\d+)?)|(["'])((?:(?!\2)[^\\]|\\.)*?)\2)\]|(?=(?:\.|\[\])(?:\.|\[\]|%$))/g, V = /\\(\\)?/g, ue = function(F) {
    var B = oe(F, 0, 1), T = oe(F, -1);
    if (B === "%" && T !== "%")
      throw new P("invalid intrinsic syntax, expected closing `%`");
    if (T === "%" && B !== "%")
      throw new P("invalid intrinsic syntax, expected opening `%`");
    var U = [];
    return ae(F, ne, function(H, J, k, j) {
      U[U.length] = k ? ae(j, V, "$1") : J || H;
    }), U;
  }, fe = function(F, B) {
    var T = F, U;
    if (K(re, T) && (U = re[T], T = "%" + U[0] + "%"), K(L, T)) {
      var H = L[T];
      if (H === R && (H = ee(T)), typeof H > "u" && !B)
        throw new p("intrinsic " + F + " exists, but is not available. Please file an issue!");
      return {
        alias: U,
        name: T,
        value: H
      };
    }
    throw new P("intrinsic " + F + " does not exist!");
  };
  return br = function(F, B) {
    if (typeof F != "string" || F.length === 0)
      throw new p("intrinsic name must be a non-empty string");
    if (arguments.length > 1 && typeof B != "boolean")
      throw new p('"allowMissing" argument must be a boolean');
    if (ie(/^%?[^%]*%?$/, F) === null)
      throw new P("`%` may not be present anywhere but at the beginning and end of the intrinsic name");
    var T = ue(F), U = T.length > 0 ? T[0] : "", H = fe("%" + U + "%", B), J = H.name, k = H.value, j = !1, X = H.alias;
    X && (U = X[0], se(T, te([0, 1], X)));
    for (var de = 1, le = !0; de < T.length; de += 1) {
      var Y = T[de], ve = oe(Y, 0, 1), he = oe(Y, -1);
      if ((ve === '"' || ve === "'" || ve === "`" || he === '"' || he === "'" || he === "`") && ve !== he)
        throw new P("property names with quotes must have matching quotes");
      if ((Y === "constructor" || !le) && (j = !0), U += "." + Y, J = "%" + U + "%", K(L, J))
        k = L[J];
      else if (k != null) {
        if (!(Y in k)) {
          if (!B)
            throw new p("base intrinsic for " + F + " exists, but the property is not available.");
          return;
        }
        if (o && de + 1 >= T.length) {
          var ge = o(k, Y);
          le = !!ge, le && "get" in ge && !("originalValue" in ge.get) ? k = ge.get : k = k[Y];
        } else
          le = K(k, Y), k = k[Y];
        le && !j && (L[J] = k);
      }
    }
    return k;
  }, br;
}
var wr, qt;
function jt() {
  if (qt) return wr;
  qt = 1;
  var t = /* @__PURE__ */ Dr(), l = kt(), f = l([t("%String.prototype.indexOf%")]);
  return wr = function(S, w) {
    var P = (
      /** @type {Parameters<typeof callBindBasic>[0][0]} */
      t(S, !!w)
    );
    return typeof P == "function" && f(S, ".prototype.") > -1 ? l([P]) : P;
  }, wr;
}
var Ar, _t;
function zt() {
  if (_t) return Ar;
  _t = 1;
  var t = /* @__PURE__ */ Dr(), l = /* @__PURE__ */ jt(), f = /* @__PURE__ */ qe(), u = /* @__PURE__ */ be(), S = t("%Map%", !0), w = l("Map.prototype.get", !0), P = l("Map.prototype.set", !0), p = l("Map.prototype.has", !0), s = l("Map.prototype.delete", !0), v = l("Map.prototype.size", !0);
  return Ar = !!S && /** @type {Exclude<import('.'), false>} */
  function() {
    var h, g = {
      assert: function(i) {
        if (!g.has(i))
          throw new u("Side channel does not contain " + f(i));
      },
      delete: function(i) {
        if (h) {
          var c = s(h, i);
          return v(h) === 0 && (h = void 0), c;
        }
        return !1;
      },
      get: function(i) {
        if (h)
          return w(h, i);
      },
      has: function(i) {
        return h ? p(h, i) : !1;
      },
      set: function(i, c) {
        h || (h = new S()), P(h, i, c);
      }
    };
    return g;
  }, Ar;
}
var Or, xt;
function Dn() {
  if (xt) return Or;
  xt = 1;
  var t = /* @__PURE__ */ Dr(), l = /* @__PURE__ */ jt(), f = /* @__PURE__ */ qe(), u = zt(), S = /* @__PURE__ */ be(), w = t("%WeakMap%", !0), P = l("WeakMap.prototype.get", !0), p = l("WeakMap.prototype.set", !0), s = l("WeakMap.prototype.has", !0), v = l("WeakMap.prototype.delete", !0);
  return Or = w ? (
    /** @type {Exclude<import('.'), false>} */
    function() {
      var h, g, i = {
        assert: function(c) {
          if (!i.has(c))
            throw new S("Side channel does not contain " + f(c));
        },
        delete: function(c) {
          if (w && c && (typeof c == "object" || typeof c == "function")) {
            if (h)
              return v(h, c);
          } else if (u && g)
            return g.delete(c);
          return !1;
        },
        get: function(c) {
          return w && c && (typeof c == "object" || typeof c == "function") && h ? P(h, c) : g && g.get(c);
        },
        has: function(c) {
          return w && c && (typeof c == "object" || typeof c == "function") && h ? s(h, c) : !!g && g.has(c);
        },
        set: function(c, b) {
          w && c && (typeof c == "object" || typeof c == "function") ? (h || (h = new w()), p(h, c, b)) : u && (g || (g = u()), g.set(c, b));
        }
      };
      return i;
    }
  ) : u, Or;
}
var Er, It;
function Cn() {
  if (It) return Er;
  It = 1;
  var t = /* @__PURE__ */ be(), l = /* @__PURE__ */ qe(), f = un(), u = zt(), S = Dn(), w = S || u || f;
  return Er = function() {
    var p, s = {
      assert: function(v) {
        if (!s.has(v))
          throw new t("Side channel does not contain " + l(v));
      },
      delete: function(v) {
        return !!p && p.delete(v);
      },
      get: function(v) {
        return p && p.get(v);
      },
      has: function(v) {
        return !!p && p.has(v);
      },
      set: function(v, d) {
        p || (p = w()), p.set(v, d);
      }
    };
    return s;
  }, Er;
}
var Pr, Ft;
function Cr() {
  if (Ft) return Pr;
  Ft = 1;
  var t = String.prototype.replace, l = /%20/g, f = {
    RFC1738: "RFC1738",
    RFC3986: "RFC3986"
  };
  return Pr = {
    default: f.RFC3986,
    formatters: {
      RFC1738: function(u) {
        return t.call(u, l, "+");
      },
      RFC3986: function(u) {
        return String(u);
      }
    },
    RFC1738: f.RFC1738,
    RFC3986: f.RFC3986
  }, Pr;
}
var Rr, Dt;
function Kt() {
  if (Dt) return Rr;
  Dt = 1;
  var t = /* @__PURE__ */ Cr(), l = Object.prototype.hasOwnProperty, f = Array.isArray, u = function() {
    for (var n = [], a = 0; a < 256; ++a)
      n.push("%" + ((a < 16 ? "0" : "") + a.toString(16)).toUpperCase());
    return n;
  }(), S = function(a) {
    for (; a.length > 1; ) {
      var o = a.pop(), m = o.obj[o.prop];
      if (f(m)) {
        for (var y = [], A = 0; A < m.length; ++A)
          typeof m[A] < "u" && y.push(m[A]);
        o.obj[o.prop] = y;
      }
    }
  }, w = function(a, o) {
    for (var m = o && o.plainObjects ? { __proto__: null } : {}, y = 0; y < a.length; ++y)
      typeof a[y] < "u" && (m[y] = a[y]);
    return m;
  }, P = function n(a, o, m) {
    if (!o)
      return a;
    if (typeof o != "object" && typeof o != "function") {
      if (f(a))
        a.push(o);
      else if (a && typeof a == "object")
        (m && (m.plainObjects || m.allowPrototypes) || !l.call(Object.prototype, o)) && (a[o] = !0);
      else
        return [a, o];
      return a;
    }
    if (!a || typeof a != "object")
      return [a].concat(o);
    var y = a;
    return f(a) && !f(o) && (y = w(a, m)), f(a) && f(o) ? (o.forEach(function(A, O) {
      if (l.call(a, O)) {
        var E = a[O];
        E && typeof E == "object" && A && typeof A == "object" ? a[O] = n(E, A, m) : a.push(A);
      } else
        a[O] = A;
    }), a) : Object.keys(o).reduce(function(A, O) {
      var E = o[O];
      return l.call(A, O) ? A[O] = n(A[O], E, m) : A[O] = E, A;
    }, y);
  }, p = function(a, o) {
    return Object.keys(o).reduce(function(m, y) {
      return m[y] = o[y], m;
    }, a);
  }, s = function(n, a, o) {
    var m = n.replace(/\+/g, " ");
    if (o === "iso-8859-1")
      return m.replace(/%[0-9a-f]{2}/gi, unescape);
    try {
      return decodeURIComponent(m);
    } catch {
      return m;
    }
  }, v = 1024, d = function(a, o, m, y, A) {
    if (a.length === 0)
      return a;
    var O = a;
    if (typeof a == "symbol" ? O = Symbol.prototype.toString.call(a) : typeof a != "string" && (O = String(a)), m === "iso-8859-1")
      return escape(O).replace(/%u[0-9a-f]{4}/gi, function(G) {
        return "%26%23" + parseInt(G.slice(2), 16) + "%3B";
      });
    for (var E = "", q = 0; q < O.length; q += v) {
      for (var C = O.length >= v ? O.slice(q, q + v) : O, _ = [], M = 0; M < C.length; ++M) {
        var R = C.charCodeAt(M);
        if (R === 45 || R === 46 || R === 95 || R === 126 || R >= 48 && R <= 57 || R >= 65 && R <= 90 || R >= 97 && R <= 122 || A === t.RFC1738 && (R === 40 || R === 41)) {
          _[_.length] = C.charAt(M);
          continue;
        }
        if (R < 128) {
          _[_.length] = u[R];
          continue;
        }
        if (R < 2048) {
          _[_.length] = u[192 | R >> 6] + u[128 | R & 63];
          continue;
        }
        if (R < 55296 || R >= 57344) {
          _[_.length] = u[224 | R >> 12] + u[128 | R >> 6 & 63] + u[128 | R & 63];
          continue;
        }
        M += 1, R = 65536 + ((R & 1023) << 10 | C.charCodeAt(M) & 1023), _[_.length] = u[240 | R >> 18] + u[128 | R >> 12 & 63] + u[128 | R >> 6 & 63] + u[128 | R & 63];
      }
      E += _.join("");
    }
    return E;
  }, h = function(a) {
    for (var o = [{ obj: { o: a }, prop: "o" }], m = [], y = 0; y < o.length; ++y)
      for (var A = o[y], O = A.obj[A.prop], E = Object.keys(O), q = 0; q < E.length; ++q) {
        var C = E[q], _ = O[C];
        typeof _ == "object" && _ !== null && m.indexOf(_) === -1 && (o.push({ obj: O, prop: C }), m.push(_));
      }
    return S(o), a;
  }, g = function(a) {
    return Object.prototype.toString.call(a) === "[object RegExp]";
  }, i = function(a) {
    return !a || typeof a != "object" ? !1 : !!(a.constructor && a.constructor.isBuffer && a.constructor.isBuffer(a));
  }, c = function(a, o) {
    return [].concat(a, o);
  }, b = function(a, o) {
    if (f(a)) {
      for (var m = [], y = 0; y < a.length; y += 1)
        m.push(o(a[y]));
      return m;
    }
    return o(a);
  };
  return Rr = {
    arrayToObject: w,
    assign: p,
    combine: c,
    compact: h,
    decode: s,
    encode: d,
    isBuffer: i,
    isRegExp: g,
    maybeMap: b,
    merge: P
  }, Rr;
}
var qr, Ct;
function Mn() {
  if (Ct) return qr;
  Ct = 1;
  var t = Cn(), l = /* @__PURE__ */ Kt(), f = /* @__PURE__ */ Cr(), u = Object.prototype.hasOwnProperty, S = {
    brackets: function(n) {
      return n + "[]";
    },
    comma: "comma",
    indices: function(n, a) {
      return n + "[" + a + "]";
    },
    repeat: function(n) {
      return n;
    }
  }, w = Array.isArray, P = Array.prototype.push, p = function(b, n) {
    P.apply(b, w(n) ? n : [n]);
  }, s = Date.prototype.toISOString, v = f.default, d = {
    addQueryPrefix: !1,
    allowDots: !1,
    allowEmptyArrays: !1,
    arrayFormat: "indices",
    charset: "utf-8",
    charsetSentinel: !1,
    commaRoundTrip: !1,
    delimiter: "&",
    encode: !0,
    encodeDotInKeys: !1,
    encoder: l.encode,
    encodeValuesOnly: !1,
    filter: void 0,
    format: v,
    formatter: f.formatters[v],
    // deprecated
    indices: !1,
    serializeDate: function(n) {
      return s.call(n);
    },
    skipNulls: !1,
    strictNullHandling: !1
  }, h = function(n) {
    return typeof n == "string" || typeof n == "number" || typeof n == "boolean" || typeof n == "symbol" || typeof n == "bigint";
  }, g = {}, i = function b(n, a, o, m, y, A, O, E, q, C, _, M, R, G, L, z, ee, re) {
    for (var I = n, K = re, te = 0, se = !1; (K = K.get(g)) !== void 0 && !se; ) {
      var ae = K.get(n);
      if (te += 1, typeof ae < "u") {
        if (ae === te)
          throw new RangeError("Cyclic object value");
        se = !0;
      }
      typeof K.get(g) > "u" && (te = 0);
    }
    if (typeof C == "function" ? I = C(a, I) : I instanceof Date ? I = R(I) : o === "comma" && w(I) && (I = l.maybeMap(I, function(J) {
      return J instanceof Date ? R(J) : J;
    })), I === null) {
      if (A)
        return q && !z ? q(a, d.encoder, ee, "key", G) : a;
      I = "";
    }
    if (h(I) || l.isBuffer(I)) {
      if (q) {
        var oe = z ? a : q(a, d.encoder, ee, "key", G);
        return [L(oe) + "=" + L(q(I, d.encoder, ee, "value", G))];
      }
      return [L(a) + "=" + L(String(I))];
    }
    var ie = [];
    if (typeof I > "u")
      return ie;
    var ne;
    if (o === "comma" && w(I))
      z && q && (I = l.maybeMap(I, q)), ne = [{ value: I.length > 0 ? I.join(",") || null : void 0 }];
    else if (w(C))
      ne = C;
    else {
      var V = Object.keys(I);
      ne = _ ? V.sort(_) : V;
    }
    var ue = E ? String(a).replace(/\./g, "%2E") : String(a), fe = m && w(I) && I.length === 1 ? ue + "[]" : ue;
    if (y && w(I) && I.length === 0)
      return fe + "[]";
    for (var W = 0; W < ne.length; ++W) {
      var F = ne[W], B = typeof F == "object" && F && typeof F.value < "u" ? F.value : I[F];
      if (!(O && B === null)) {
        var T = M && E ? String(F).replace(/\./g, "%2E") : String(F), U = w(I) ? typeof o == "function" ? o(fe, T) : fe : fe + (M ? "." + T : "[" + T + "]");
        re.set(n, te);
        var H = t();
        H.set(g, re), p(ie, b(
          B,
          U,
          o,
          m,
          y,
          A,
          O,
          E,
          o === "comma" && z && w(I) ? null : q,
          C,
          _,
          M,
          R,
          G,
          L,
          z,
          ee,
          H
        ));
      }
    }
    return ie;
  }, c = function(n) {
    if (!n)
      return d;
    if (typeof n.allowEmptyArrays < "u" && typeof n.allowEmptyArrays != "boolean")
      throw new TypeError("`allowEmptyArrays` option can only be `true` or `false`, when provided");
    if (typeof n.encodeDotInKeys < "u" && typeof n.encodeDotInKeys != "boolean")
      throw new TypeError("`encodeDotInKeys` option can only be `true` or `false`, when provided");
    if (n.encoder !== null && typeof n.encoder < "u" && typeof n.encoder != "function")
      throw new TypeError("Encoder has to be a function.");
    var a = n.charset || d.charset;
    if (typeof n.charset < "u" && n.charset !== "utf-8" && n.charset !== "iso-8859-1")
      throw new TypeError("The charset option must be either utf-8, iso-8859-1, or undefined");
    var o = f.default;
    if (typeof n.format < "u") {
      if (!u.call(f.formatters, n.format))
        throw new TypeError("Unknown format option provided.");
      o = n.format;
    }
    var m = f.formatters[o], y = d.filter;
    (typeof n.filter == "function" || w(n.filter)) && (y = n.filter);
    var A;
    if (n.arrayFormat in S ? A = n.arrayFormat : "indices" in n ? A = n.indices ? "indices" : "repeat" : A = d.arrayFormat, "commaRoundTrip" in n && typeof n.commaRoundTrip != "boolean")
      throw new TypeError("`commaRoundTrip` must be a boolean, or absent");
    var O = typeof n.allowDots > "u" ? n.encodeDotInKeys === !0 ? !0 : d.allowDots : !!n.allowDots;
    return {
      addQueryPrefix: typeof n.addQueryPrefix == "boolean" ? n.addQueryPrefix : d.addQueryPrefix,
      allowDots: O,
      allowEmptyArrays: typeof n.allowEmptyArrays == "boolean" ? !!n.allowEmptyArrays : d.allowEmptyArrays,
      arrayFormat: A,
      charset: a,
      charsetSentinel: typeof n.charsetSentinel == "boolean" ? n.charsetSentinel : d.charsetSentinel,
      commaRoundTrip: !!n.commaRoundTrip,
      delimiter: typeof n.delimiter > "u" ? d.delimiter : n.delimiter,
      encode: typeof n.encode == "boolean" ? n.encode : d.encode,
      encodeDotInKeys: typeof n.encodeDotInKeys == "boolean" ? n.encodeDotInKeys : d.encodeDotInKeys,
      encoder: typeof n.encoder == "function" ? n.encoder : d.encoder,
      encodeValuesOnly: typeof n.encodeValuesOnly == "boolean" ? n.encodeValuesOnly : d.encodeValuesOnly,
      filter: y,
      format: o,
      formatter: m,
      serializeDate: typeof n.serializeDate == "function" ? n.serializeDate : d.serializeDate,
      skipNulls: typeof n.skipNulls == "boolean" ? n.skipNulls : d.skipNulls,
      sort: typeof n.sort == "function" ? n.sort : null,
      strictNullHandling: typeof n.strictNullHandling == "boolean" ? n.strictNullHandling : d.strictNullHandling
    };
  };
  return qr = function(b, n) {
    var a = b, o = c(n), m, y;
    typeof o.filter == "function" ? (y = o.filter, a = y("", a)) : w(o.filter) && (y = o.filter, m = y);
    var A = [];
    if (typeof a != "object" || a === null)
      return "";
    var O = S[o.arrayFormat], E = O === "comma" && o.commaRoundTrip;
    m || (m = Object.keys(a)), o.sort && m.sort(o.sort);
    for (var q = t(), C = 0; C < m.length; ++C) {
      var _ = m[C], M = a[_];
      o.skipNulls && M === null || p(A, i(
        M,
        _,
        O,
        E,
        o.allowEmptyArrays,
        o.strictNullHandling,
        o.skipNulls,
        o.encodeDotInKeys,
        o.encode ? o.encoder : null,
        o.filter,
        o.sort,
        o.allowDots,
        o.serializeDate,
        o.format,
        o.formatter,
        o.encodeValuesOnly,
        o.charset,
        q
      ));
    }
    var R = A.join(o.delimiter), G = o.addQueryPrefix === !0 ? "?" : "";
    return o.charsetSentinel && (o.charset === "iso-8859-1" ? G += "utf8=%26%2310003%3B&" : G += "utf8=%E2%9C%93&"), R.length > 0 ? G + R : "";
  }, qr;
}
var _r, Mt;
function Tn() {
  if (Mt) return _r;
  Mt = 1;
  var t = /* @__PURE__ */ Kt(), l = Object.prototype.hasOwnProperty, f = Array.isArray, u = {
    allowDots: !1,
    allowEmptyArrays: !1,
    allowPrototypes: !1,
    allowSparse: !1,
    arrayLimit: 20,
    charset: "utf-8",
    charsetSentinel: !1,
    comma: !1,
    decodeDotInKeys: !1,
    decoder: t.decode,
    delimiter: "&",
    depth: 5,
    duplicates: "combine",
    ignoreQueryPrefix: !1,
    interpretNumericEntities: !1,
    parameterLimit: 1e3,
    parseArrays: !0,
    plainObjects: !1,
    strictDepth: !1,
    strictNullHandling: !1,
    throwOnLimitExceeded: !1
  }, S = function(g) {
    return g.replace(/&#(\d+);/g, function(i, c) {
      return String.fromCharCode(parseInt(c, 10));
    });
  }, w = function(g, i, c) {
    if (g && typeof g == "string" && i.comma && g.indexOf(",") > -1)
      return g.split(",");
    if (i.throwOnLimitExceeded && c >= i.arrayLimit)
      throw new RangeError("Array limit exceeded. Only " + i.arrayLimit + " element" + (i.arrayLimit === 1 ? "" : "s") + " allowed in an array.");
    return g;
  }, P = "utf8=%26%2310003%3B", p = "utf8=%E2%9C%93", s = function(i, c) {
    var b = { __proto__: null }, n = c.ignoreQueryPrefix ? i.replace(/^\?/, "") : i;
    n = n.replace(/%5B/gi, "[").replace(/%5D/gi, "]");
    var a = c.parameterLimit === 1 / 0 ? void 0 : c.parameterLimit, o = n.split(
      c.delimiter,
      c.throwOnLimitExceeded ? a + 1 : a
    );
    if (c.throwOnLimitExceeded && o.length > a)
      throw new RangeError("Parameter limit exceeded. Only " + a + " parameter" + (a === 1 ? "" : "s") + " allowed.");
    var m = -1, y, A = c.charset;
    if (c.charsetSentinel)
      for (y = 0; y < o.length; ++y)
        o[y].indexOf("utf8=") === 0 && (o[y] === p ? A = "utf-8" : o[y] === P && (A = "iso-8859-1"), m = y, y = o.length);
    for (y = 0; y < o.length; ++y)
      if (y !== m) {
        var O = o[y], E = O.indexOf("]="), q = E === -1 ? O.indexOf("=") : E + 1, C, _;
        q === -1 ? (C = c.decoder(O, u.decoder, A, "key"), _ = c.strictNullHandling ? null : "") : (C = c.decoder(O.slice(0, q), u.decoder, A, "key"), _ = t.maybeMap(
          w(
            O.slice(q + 1),
            c,
            f(b[C]) ? b[C].length : 0
          ),
          function(R) {
            return c.decoder(R, u.decoder, A, "value");
          }
        )), _ && c.interpretNumericEntities && A === "iso-8859-1" && (_ = S(String(_))), O.indexOf("[]=") > -1 && (_ = f(_) ? [_] : _);
        var M = l.call(b, C);
        M && c.duplicates === "combine" ? b[C] = t.combine(b[C], _) : (!M || c.duplicates === "last") && (b[C] = _);
      }
    return b;
  }, v = function(g, i, c, b) {
    var n = 0;
    if (g.length > 0 && g[g.length - 1] === "[]") {
      var a = g.slice(0, -1).join("");
      n = Array.isArray(i) && i[a] ? i[a].length : 0;
    }
    for (var o = b ? i : w(i, c, n), m = g.length - 1; m >= 0; --m) {
      var y, A = g[m];
      if (A === "[]" && c.parseArrays)
        y = c.allowEmptyArrays && (o === "" || c.strictNullHandling && o === null) ? [] : t.combine([], o);
      else {
        y = c.plainObjects ? { __proto__: null } : {};
        var O = A.charAt(0) === "[" && A.charAt(A.length - 1) === "]" ? A.slice(1, -1) : A, E = c.decodeDotInKeys ? O.replace(/%2E/g, ".") : O, q = parseInt(E, 10);
        !c.parseArrays && E === "" ? y = { 0: o } : !isNaN(q) && A !== E && String(q) === E && q >= 0 && c.parseArrays && q <= c.arrayLimit ? (y = [], y[q] = o) : E !== "__proto__" && (y[E] = o);
      }
      o = y;
    }
    return o;
  }, d = function(i, c, b, n) {
    if (i) {
      var a = b.allowDots ? i.replace(/\.([^.[]+)/g, "[$1]") : i, o = /(\[[^[\]]*])/, m = /(\[[^[\]]*])/g, y = b.depth > 0 && o.exec(a), A = y ? a.slice(0, y.index) : a, O = [];
      if (A) {
        if (!b.plainObjects && l.call(Object.prototype, A) && !b.allowPrototypes)
          return;
        O.push(A);
      }
      for (var E = 0; b.depth > 0 && (y = m.exec(a)) !== null && E < b.depth; ) {
        if (E += 1, !b.plainObjects && l.call(Object.prototype, y[1].slice(1, -1)) && !b.allowPrototypes)
          return;
        O.push(y[1]);
      }
      if (y) {
        if (b.strictDepth === !0)
          throw new RangeError("Input depth exceeded depth option of " + b.depth + " and strictDepth is true");
        O.push("[" + a.slice(y.index) + "]");
      }
      return v(O, c, b, n);
    }
  }, h = function(i) {
    if (!i)
      return u;
    if (typeof i.allowEmptyArrays < "u" && typeof i.allowEmptyArrays != "boolean")
      throw new TypeError("`allowEmptyArrays` option can only be `true` or `false`, when provided");
    if (typeof i.decodeDotInKeys < "u" && typeof i.decodeDotInKeys != "boolean")
      throw new TypeError("`decodeDotInKeys` option can only be `true` or `false`, when provided");
    if (i.decoder !== null && typeof i.decoder < "u" && typeof i.decoder != "function")
      throw new TypeError("Decoder has to be a function.");
    if (typeof i.charset < "u" && i.charset !== "utf-8" && i.charset !== "iso-8859-1")
      throw new TypeError("The charset option must be either utf-8, iso-8859-1, or undefined");
    if (typeof i.throwOnLimitExceeded < "u" && typeof i.throwOnLimitExceeded != "boolean")
      throw new TypeError("`throwOnLimitExceeded` option must be a boolean");
    var c = typeof i.charset > "u" ? u.charset : i.charset, b = typeof i.duplicates > "u" ? u.duplicates : i.duplicates;
    if (b !== "combine" && b !== "first" && b !== "last")
      throw new TypeError("The duplicates option must be either combine, first, or last");
    var n = typeof i.allowDots > "u" ? i.decodeDotInKeys === !0 ? !0 : u.allowDots : !!i.allowDots;
    return {
      allowDots: n,
      allowEmptyArrays: typeof i.allowEmptyArrays == "boolean" ? !!i.allowEmptyArrays : u.allowEmptyArrays,
      allowPrototypes: typeof i.allowPrototypes == "boolean" ? i.allowPrototypes : u.allowPrototypes,
      allowSparse: typeof i.allowSparse == "boolean" ? i.allowSparse : u.allowSparse,
      arrayLimit: typeof i.arrayLimit == "number" ? i.arrayLimit : u.arrayLimit,
      charset: c,
      charsetSentinel: typeof i.charsetSentinel == "boolean" ? i.charsetSentinel : u.charsetSentinel,
      comma: typeof i.comma == "boolean" ? i.comma : u.comma,
      decodeDotInKeys: typeof i.decodeDotInKeys == "boolean" ? i.decodeDotInKeys : u.decodeDotInKeys,
      decoder: typeof i.decoder == "function" ? i.decoder : u.decoder,
      delimiter: typeof i.delimiter == "string" || t.isRegExp(i.delimiter) ? i.delimiter : u.delimiter,
      // eslint-disable-next-line no-implicit-coercion, no-extra-parens
      depth: typeof i.depth == "number" || i.depth === !1 ? +i.depth : u.depth,
      duplicates: b,
      ignoreQueryPrefix: i.ignoreQueryPrefix === !0,
      interpretNumericEntities: typeof i.interpretNumericEntities == "boolean" ? i.interpretNumericEntities : u.interpretNumericEntities,
      parameterLimit: typeof i.parameterLimit == "number" ? i.parameterLimit : u.parameterLimit,
      parseArrays: i.parseArrays !== !1,
      plainObjects: typeof i.plainObjects == "boolean" ? i.plainObjects : u.plainObjects,
      strictDepth: typeof i.strictDepth == "boolean" ? !!i.strictDepth : u.strictDepth,
      strictNullHandling: typeof i.strictNullHandling == "boolean" ? i.strictNullHandling : u.strictNullHandling,
      throwOnLimitExceeded: typeof i.throwOnLimitExceeded == "boolean" ? i.throwOnLimitExceeded : !1
    };
  };
  return _r = function(g, i) {
    var c = h(i);
    if (g === "" || g === null || typeof g > "u")
      return c.plainObjects ? { __proto__: null } : {};
    for (var b = typeof g == "string" ? s(g, c) : g, n = c.plainObjects ? { __proto__: null } : {}, a = Object.keys(b), o = 0; o < a.length; ++o) {
      var m = a[o], y = d(m, b[m], c, typeof g == "string");
      n = t.merge(n, y, c);
    }
    return c.allowSparse === !0 ? n : t.compact(n);
  }, _r;
}
var xr, Tt;
function $n() {
  if (Tt) return xr;
  Tt = 1;
  var t = /* @__PURE__ */ Mn(), l = /* @__PURE__ */ Tn(), f = /* @__PURE__ */ Cr();
  return xr = {
    formats: f,
    parse: l,
    stringify: t
  }, xr;
}
var $t;
function Nn() {
  return $t || ($t = 1, function(t) {
    t.__esModule = !0;
    var l = /* @__PURE__ */ $n();
    function f(u, S) {
      var w = u.split("?"), P = w[0], p = w[1], s = (p || "").split("#")[0], v = p && p.split("#").length > 1 ? "#" + p.split("#")[1] : "", d = l.parse(s);
      for (var h in S)
        d[h] = S[h];
      return s = l.stringify(d), s !== "" && (s = "?" + s), P + s + v;
    }
    t.default = f;
  }(Be)), Be;
}
var Ir, Nt;
function Bn() {
  if (Nt) return Ir;
  Nt = 1;
  var t = (
    /** @class */
    function() {
      function l(f, u, S, w) {
        if (typeof f != "number")
          throw new TypeError("statusCode must be a number but was " + typeof f);
        if (u === null)
          throw new TypeError("headers cannot be null");
        if (typeof u != "object")
          throw new TypeError("headers must be an object but was " + typeof u);
        this.statusCode = f;
        var P = {};
        for (var p in u)
          P[p.toLowerCase()] = u[p];
        this.headers = P, this.body = S, this.url = w;
      }
      return l.prototype.isError = function() {
        return this.statusCode === 0 || this.statusCode >= 400;
      }, l.prototype.getBody = function(f) {
        if (this.statusCode === 0) {
          var u = new Error("This request to " + this.url + ` resulted in a status code of 0. This usually indicates some kind of network error in a browser (e.g. CORS not being set up or the DNS failing to resolve):
` + this.body.toString());
          throw u.statusCode = this.statusCode, u.headers = this.headers, u.body = this.body, u.url = this.url, u;
        }
        if (this.statusCode >= 300) {
          var u = new Error("Server responded to " + this.url + " with status code " + this.statusCode + `:
` + this.body.toString());
          throw u.statusCode = this.statusCode, u.headers = this.headers, u.body = this.body, u.url = this.url, u;
        }
        return !f || typeof this.body == "string" ? this.body : this.body.toString(f);
      }, l;
    }()
  );
  return Ir = t, Ir;
}
var Bt;
function Ln() {
  return Bt || (Bt = 1, function(t, l) {
    l.__esModule = !0;
    var f = Nn(), u = Bn(), S = FormData;
    l.FormData = S;
    function w(P, p, s) {
      var v = new XMLHttpRequest();
      if (typeof P != "string")
        throw new TypeError("The method must be a string.");
      if (p && typeof p == "object" && (p = p.href), typeof p != "string")
        throw new TypeError("The URL/path must be a string.");
      if (s == null && (s = {}), typeof s != "object")
        throw new TypeError("Options must be an object (or null).");
      P = P.toUpperCase(), s.headers = s.headers || {};
      var d, h = !!((d = /^([\w-]+:)?\/\/([^\/]+)/.exec(p)) && d[2] != location.host);
      h || (s.headers["X-Requested-With"] = "XMLHttpRequest"), s.qs && (p = f.default(p, s.qs)), s.json && (s.body = JSON.stringify(s.json), s.headers["content-type"] = "application/json"), s.form && (s.body = s.form), v.open(P, p, !1);
      for (var g in s.headers)
        v.setRequestHeader(g.toLowerCase(), "" + s.headers[g]);
      v.send(s.body ? s.body : null);
      var i = {};
      return v.getAllResponseHeaders().split(`\r
`).forEach(function(c) {
        var b = c.split(":");
        b.length > 1 && (i[b[0].toLowerCase()] = b.slice(1).join(":").trim());
      }), new u(v.status, i, v.responseText, p);
    }
    l.default = w, t.exports = w, t.exports.default = w, t.exports.FormData = S;
  }(Re, Re.exports)), Re.exports;
}
var Un = Ln();
const Wn = /* @__PURE__ */ rn(Un), Gn = new TextEncoder();
function Hn() {
  if (typeof process < "u" && process.env != null)
    return process.env.HIRO_API_KEY;
}
function kn(t, l) {
  const f = {
    headers: {
      "x-hiro-product": "clarinet-sdk"
    }
  }, u = Hn();
  u && (f.headers["x-api-key"] = u);
  const S = Wn(t, l, f);
  return typeof S.body == "string" ? { status: S.statusCode, body: Gn.encode(S.body) } : {
    status: S.statusCode,
    body: new Uint8Array(S.body)
  };
}
export {
  Hn as getHiroApiKey,
  kn as httpClient
};
