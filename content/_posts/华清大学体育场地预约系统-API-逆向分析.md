---
title: 华清大学体育场地预约系统 API 逆向分析
tags:
  - 逆向
  - API
  - Python
categories:
  - 技术
mathjax: false
toc: true
date: 2026-06-09 14:56:38
password:
id: Tsnighua-Sports-Venue-Reservation-API-Reverse-Engineering
---

[华清大学体育场地预约系统](https://www.sports.tsinghua.edu.cn/venue/#/home) 的北体育馆中的乒乓球场地在 2026 年马约翰杯乒乓球比赛前后被严格监控是否预约，以至于需要线下签到。对于系乒乓球队而言，这无疑是非常困难的，因为每个人在同一时段只能预约一张球台；而系队训练一般需要多张。因此，一般是收集多个同学的账号进行预约。

但是现在要求线下出示预约二维码进行签到；二维码的有效期为 30s，这造成了极大的困难：不能总是麻烦没来训练的同学登录系统转发二维码。因此，我们尝试开发一套工作流程，能够实时获取该二维码。

<!--more-->

整体的思路分为两步。由于（众所周知）学校的各个系统网页端的鉴权几乎都是基于 Token~~（“词元”）~~的，所以第一步是逆向分析系统 API，此时 **假设我们已有合法 Token**。第二步再是实现自动登录获取 Token，以及免二次验证状态持久化。

我们的唯一目标是 **获取预约二维码**。

## 先抓二维码请求

从页面入口看，最直接的线索是“我的预约”里的二维码按钮。打开 DevTools 的 Network 面板，点开二维码，马上能看到一个很可疑的请求：

```bash
curl 'https://www.sports.tsinghua.edu.cn/venue/site/api/reserve/user/qr?appId=1497016617475903488&timeStamp=1781318962679&nonce=Z9EMAFcZAtEGTyjRnpZNCzDSsYfbFf3x&sign=dea17cf8424a12a86713cb1091959327' \
  -H 'token: <JWT>' \
  ...
```

接口返回的 JSON 很短：

```json
{
  "code": 0,
  "message": "请求成功",
  "success": true,
  "data": "unil1SyG..."
}
```

这里的 `data` 不是图片地址，也不是 Base64 图片，而是二维码里实际编码的字符串。前端拿到这段字符串之后，直接交给 QR Code 组件渲染将这个字符串展示为二维码。

这样一来，问题就集中到 URL 后面的四个查询参数上：

| 参数 | 说明 |
|---|---|
| `appId` | 固定值 `"1497016617475903488"` |
| `timeStamp` | 当前毫秒时间戳 |
| `nonce` | 32 位随机字符串 |
| `sign` | 请求签名 |

多抓几次就能看出来 `appId` 是固定的；`timeStamp` 的数量级和当前毫秒时间戳一致；`nonce` 每次变化，而且长度稳定为 32。剩下的 `sign` 最关键，也最不可能靠猜。它通常会由前面几个字段加上某个密钥算出来，所以接下来要回到前端代码里找签名逻辑。

## 找到签名函数

前端资源是打包后的 JS，文件名带 hash。我当时看到的是 `index.47b9a9bc.js`，以后重新部署后名字可能会变，所以文件名本身不重要，关键是搜索词。

我先搜了几个最直接的关键词，`sign`、`appId`、`nonce` 和 `timeStamp`。

签名逻辑一般会放在统一请求封装里，因为每个接口都要加同一套公共参数。顺着 `appId` 和 `sign` 往附近看，可以看到类似下面的模块：

```javascript
b764: function(e, n, t) {
  "use strict";
  t("6a54");
  var i = t("f5bd").default;
  Object.defineProperty(n, "__esModule", {
      value: !0
  }),
  n.API_SIGN = void 0;
  var a = i(t("8078"))
    , o = t("e4ea")
    , s = {
      randomString: function(e) {
          for (var n = "ABCDEFGHJKMNPQRSTWXYZabcdefhijkmnprstwxyz0123456789", t = n.length, i = "", a = 0; a < e; a++)
              i += n.charAt(Math.floor(Math.random() * t));
          return i
      },
      getSign: function() {
          var e = "1497016617475903488"
            , n = (0,
          o.getKeys)().join("")
            , t = (new Date).getTime()
            , i = this.randomString(32)
            , s = "appId=" + e + "&nonce=" + i + "&timeStamp=" + t + "&key=" + n
            , r = (0,
          a.default)(s);
          return {
              appId: e,
              timeStamp: t,
              nonce: i,
              sign: r
          }
      }
  };
  n.API_SIGN = s
},
```

这段代码已经把签名的大体结构暴露出来了：

-   `e` 是固定的 `appId`。
-   `t` 是 `(new Date).getTime()`，也就是毫秒时间戳。
-   `i` 是长度为 32 的随机字符串。
-   `n` 来自 `getKeys().join("")`，看起来就是密钥。
-   `r = a.default(s)`，其中 `s` 是待签名字符串。

`a.default` 也需要确认一下。它来自 webpack 模块 `8078`，继续跳过去可以看到 `js-md5` 的注释：

```javascript
8078: function(t, e, n) {
  (function(t, e, r) {
    var i, o = n("bdbb").default;
    n("80e3"),
    n("4db2"),
    n("bf0f"),
    n("c976"),
    n("4d8f"),
    n("7b97"),
    n("668a"),
    n("c5b7"),
    n("8ff5"),
    n("2378"),
    n("641a"),
    n("64e0"),
    n("cce3"),
    n("efba"),
    n("d009"),
    n("bd7d"),
    n("7edd"),
    n("d798"),
    n("f547"),
    n("5e54"),
    n("b60a"),
    n("8c18"),
    n("12973"),
    n("f991"),
    n("198e"),
    n("8557"),
    n("63b1"),
    n("1954"),
    n("1cf1"),
    n("295e"),
    n("c753"),
    n("7a76"),
    n("c9b5"),
    n("ab80"),
    /**
     * [js-md5]{@link https://github.com/emn178/js-md5}
     *
     * @namespace md5
     * @version 0.8.3
     * @author Chen, Yi-Cyuan [emn178@gmail.com]
     * @copyright Chen, Yi-Cyuan 2014-2023
     * @license MIT
     */
    ...
```

所以 `a.default` 就是 MD5 函数。剩下唯一没展开的是 `getKeys()`。

## 还原密钥

继续在打包文件里搜 `getKeys`，可以找到这样一段：

```javascript
5532: function(e, n, t) {
  "use strict";
  t("6a54"),
  Object.defineProperty(n, "__esModule", {
      value: !0
  }),
  n.getKeys = void 0,
  t("aa9c"),
  t("dc69");
  n.getKeys = function() {
      for (var e = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789", n = e[28] + e[56] + e[52] + e[27] + e[29] + e[60] + e[28], t = "752769392d3907", i = "", a = [], o = 0; o < t.length; o++)
          o % 2 == 0 ? i += t[o] : a.push(t[o]);
      var s = [7, 6, 7, 5, 3, 5]
        , r = [7, 2, 9, 2, 2]
        , d = "";
      s.reverse();
      for (var c = 0; c < s.length; c++)
          d += s[c],
          void 0 != r[c] && (d += r[c]);
      a.reverse();
      for (var u = "", l = 0; l < a.length; l++)
          u += i[l],
          u += a[l];
      return [d, n, u]
  }
},
```

这段看起来绕，但本质只是把一个常量拆散再拼回去。前端真正使用的时候调用的是：

```javascript
getKeys().join("")
```

把它跑出来，结果就是：

```text
57325972627c40bd8c77296d39293705
```

到这里，签名所需的所有输入就都齐了。

## 签名算法

整理一下生成过程：

1.  `appId` 固定为 `"1497016617475903488"`。
2.  `timeStamp` 取当前毫秒时间戳。
3.  `nonce` 取 32 位随机字符串，前端使用的字符集是 `ABCDEFGHJKMNPQRSTWXYZabcdefhijkmnprstwxyz0123456789`。
4.  `key` 为 `getKeys().join("")` 的结果，即 `"57325972627c40bd8c77296d39293705"`。
5.  按下面的顺序拼接字符串：

    ```text
    appId={appId}&nonce={nonce}&timeStamp={timeStamp}&key={key}
    ```

6.  对拼接结果计算 MD5，得到 `sign`。

用 Python 写出来就是：

```python
import hashlib
import random
import time

APP_ID = "1497016617475903488"
SECRET_KEY = "57325972627c40bd8c77296d39293705"
NONCE_CHARS = "ABCDEFGHJKMNPQRSTWXYZabcdefhijkmnprstwxyz0123456789"


def generate_sign_params():
    timestamp = str(int(time.time() * 1000))
    nonce = "".join(random.choices(NONCE_CHARS, k=32))
    raw = f"appId={APP_ID}&nonce={nonce}&timeStamp={timestamp}&key={SECRET_KEY}"
    sign = hashlib.md5(raw.encode()).hexdigest()
    return {"appId": APP_ID, "timeStamp": timestamp, "nonce": nonce, "sign": sign}
```

最后拿抓包里的参数验算一次：

```text
appId=1497016617475903488
nonce=8eNiaim2GB6KQmrfb180Aa2xPYPyeQkQ
timeStamp=1776432736175
key=57325972627c40bd8c77296d39293705

MD5("appId=1497016617475903488&nonce=8eNiaim2GB6KQmrfb180Aa2xPYPyeQkQ&timeStamp=1776432736175&key=57325972627c40bd8c77296d39293705")
= 3f9495413a1e30470afc7691a39ee1c9
```

结果和原请求中的 `sign` 对得上。至此，我们已经完全还原了二维码接口的签名算法。

不过，需要注意的是，二维码本身仍然是**短期**有效的。前端只是把后端返回的 `data` 渲染出来，真正的有效期判断大概率发生在扫码签到的服务端逻辑里，估计是那一端限制了二维码的有效期为 30s。
