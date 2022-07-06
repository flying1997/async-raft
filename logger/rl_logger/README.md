Rlogger共有五个日志等级，从低到高依次为：Trace, Debug, Info, Warn, Error
现给出使用规则(V1.0)如下:
| Level | 简介 |
| :----: | :----:|
| Trace | 用于测试时，模块开发时输出调试信息用(目前建议只使用debug) |
| Debug | 用于测试时，系统整体运行测试，输出调试信息 |
| Info | 用于生产环境中，输出对用户的有效信息，如甲方需求的共识时延等 |
| Warn | 用于输出系统异常状态，如本地超时等 |
| Error | 用于输出可能使系统崩溃的信息，如必需服务访问失败等 |

实际部署时，默认日志等级为Info
需要使用时，在包toml内加入：
`rl-logger = {path = "../crates/rl_logger",version = "0.1.0"}`
在具体使用时：
`use rl_logger::prelude::*;`
在输出时有三种使用方法：
第一种，直接输出字符串

```
trace!("trace");
info!("info");
debug!("debug");
warn!("warn");
error!("error");
// Output example:
// 20xx-xx-xx xx:xx:xx.x..x [test_logger] ERROR crates/rl_logger/test_error.rs:23  error
```
第二种，输出日志信息与参数内容
```
// Format & argument
// 20xx-xx-xx xx:xx:xx.x..x [raft] DEBUG xxx/xxx/xxx.rs:xx  Print Config {"config": [node_config's content]}
debug!(config = node_config,"Print Config");
```

第三种，格式化输出日志信息
```
// 20xx-xx-xx xx:xx:xx.x..x [module_name] INFO xxx/xxx/xxx.rs:xx Current round = 0
let round = 0;
info!("Current round = {}",round);

```
