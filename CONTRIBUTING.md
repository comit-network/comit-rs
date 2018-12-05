# Contributing

Thank you for your interest in contributing to COMIT! Contributions are welcome in many forms, and we appreciate all of them.
This document is a bit long, please find the link to the sections below:

* [Bug Reports](#bug-reports)
  * [Reporting a Security Issue](#reporting-a-security-issue)
* [Feature Requests](#feature-requests)
* [Pull Requests](#pull-requests)

## Bug Reports

No software is perfect and COMIT is no exception.
If you find a bug we would be greatly grateful if you decide to report it.

**If you believe that reporting your bug publicly represents a security or financial risk to COMIT users and developers, please refer to the [reporting a security issue](#ReportingASecurityIssue) section.**

First, please check that no user has already reported the same problem by [searching issues](/issues?q=is%3Aissue+is%3Aopen+sort%3Aupdated-desc).

To report the bug, just head over to the [create issue](/issues/new/choose) page.
Please try to provide as much information as possible by following the GitHub template.


### Reporting a Security Issue

If you think that you found a security vulnerability in the COMIT protocol, implementation or any of the smart contracts used, please send an encrypted email to [security@coblox.tech](mailto:security@coblox.tech).

Please use PGP (Pretty Good Privacy) to encrypt the report. 
The public key can be found on [coblox.tech](https://coblox.tech/security_coblox_tech_pubkey.gpg.asc), [SKS Keyservers](http://hkps.pool.sks-keyservers.net/pks/lookup?op=get&search=0xA3FE95C45DC90212) or at the end of this file.

Thank you for taking the time to make COMIT safer!


## Feature Requests

To request a change on the way COMIT works, please head over to the [RFCs repository](https://github.com/comit-network/rfc) and checkout the [RFC contributing guidelines](https://github.com/comit-network/rfc/blob/master/CONTRIBUTING.md)
 
For changes related to our implementation, please [create a feature request](/issues/new/choose) and follow the GitHub template.

## Pull Requests

If you wish to directly contribute to the code, please make pull requests against `master`.
Refer to [GitHub documentation](https://help.github.com/articles/about-pull-requests/) on using the Pull Request feature.
You can also find more details in the [Open Source Guides](https://opensource.guide/how-to-contribute/#opening-a-pull-request).

**Before** committing, always run [`cargo fmt`](https://github.com/rust-lang-nursery/rustfmt) or your change will be rejected by the CI.

To ensure you have made no breaking changes, run `cargo test --all`.

When creating commits, please follow these commit guidelines: https://chris.beams.io/posts/git-commit/.

Please be sure to double check your commit history and try to keep it clean, especially after integrating feedback.

Finally, new code needs to be accompanied by **new tests**. Please use existing tests as example.
We are using Continuous Integration and having tests will not only ensure that your code is correct but also that no-one else is breaking it unintentionally.


## Public Key
Public Key for [security@coblox.tech](mailto:security@coblox.tech)

```
-----BEGIN PGP PUBLIC KEY BLOCK-----

mQENBFufQGYBCACuZjOIZqRjVC5aI485OMMLYYqNS7c1aK3cjZUbk0eTWq9vcCMn
/I48+QAWirtznnVExyNReBtxY1kKlmSmV6WDilbDK5CmWs7OrYlJE0X1haD3+4Do
6c7ey8VcqyuZHFcpTTeb5be7pk3ZAAt6/AMy0fY40y26yoKS2Nw5/6Loh+pprDVL
wftTc8jWGsheKnLzVjdc+Db0LG+9jCi+WyCWCFPS4VKE9e/qhY7pf/tnf1/ijeUV
8JTpuOocSA88o1H4L5va/oyoT2sOnat9n9pVNzeMxeQlyQfFTonOYigrmn4lv+VT
s0TXkY+ZgJ9cTCfYrKmBrNpm6zEH76FDJEhlABEBAAG0K0NvQmxvWCBTZWN1cml0
eSBUZWFtIDxzZWN1cml0eUBjb2Jsb3gudGVjaD6JAVQEEwEIAD4WIQSzs/h+ssr7
N9/Dzf6j/pXEXckCEgUCW59AZgIbAwUJA8JnAAULCQgHAgYVCgkICwIEFgIDAQIe
AQIXgAAKCRCj/pXEXckCEmpeB/43uOA20ji/yCpDVgBZPFja/nP1C/aEiDOq8Vgy
qSJaGPzQgppucAiF2b/dSJQ81fC+areXNA9piGbS5EcAUFUTU3V2Ya7UmJBiCubU
/Rbsk6HpDVraHFMwQaxldY6eMLbd3XYTXdK7CrcVWCjA7fFzZhJVGobMxDdL4c2A
e1t0yzH+9LCuNwi1CTB5zAaxf68E/bNX4h+LrlZmgLtZAqUngtZGsgCLfJgzOGzs
GcKzYypbeBrpq56QwAuZvZ2w/KUuT19zT9X5BV7/BZZukhfw/uZ4SJJwAzKXW7uY
ZBZ17aRqJM9mJOjgO5xPmFzTuj6IgSI3fSec1xvtbF87l2qxuQENBFufQGYBCACh
9t2pehXx5Qx5y+lRBa7B8rSSoyiqep0yTYYd6FXmhIcwl3e84qHFefPf0Mh1q0l+
09DTI87zxkotIzPxVAuFA/3J+6MX/qdPvQ7KGagMe9ed+tUt/Ijk03skDeJwzwfc
hmwmIQ1UujHMkJJTJpM4Ajc5dJBggksF/O8/VF9HW6lUzj1Ap9pAf23VZDRQToyx
zO1/lxlxQMfeQEZxGmf1gtOmmL8q8nq7+UWdoJQPeRQaLTUKKorUqzjGhBc0YThV
GI7bNybrx/GD48xCECwkBvh5qJGSm6mWFf51czw3YhRxVPf2kRCFQKe0cC4JM/EQ
fuoaUWwaQMMlNCBW4oV1ABEBAAGJATwEGAEIACYWIQSzs/h+ssr7N9/Dzf6j/pXE
XckCEgUCW59AZgIbDAUJA8JnAAAKCRCj/pXEXckCEkbZB/9ek1iYKkKLwVZqNs37
/y5DKgLV+8sQHX8Y/+7+5f46rGbPaKS1evxSfG54dX9+BgZAnLq7meW+HX9oxyJI
Wv0p0xyr0vDEykAX7nYPCdIN1v9a0hPJ4uA5bY4tXCdUKPUD1T7x/MDJD196ZZaB
tRezSCxxr3WBLcaHIFsUjSYn6vxGSp7dOXicKSUphhqT+M0+A0FflC0G6XRW8+U0
olI8NzeNSr5cjGBY/CTLMVi2obc83idIesLZeqFsyOhTU5+0wpXifMlfk8bhVXi7
YGxEce3fodmoekqpUTC7Xnf3Y/7yTxuYq0hdEYI12mXdo8HL9sVuVjPnjrM/p6EU
rsqZ
=wHOK
-----END PGP PUBLIC KEY BLOCK-----
```
