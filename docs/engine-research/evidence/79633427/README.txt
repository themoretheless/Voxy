UeCore是全球著名的魔兽世界游戏引擎开源项目
==============================
UeCore(Universe Engine Core) is an Open Source MMO RPG Framework World of Warcraft Server Engine (NOT Unreal Engine)
==============================
> UeCore是唯一一个持续了十多年的长期开源的国产开源魔兽世界游戏引擎项目，该项目基于最高60级的经典旧世界资料片版本，对应的客户端版本是1.12。
> 
> UeCore能够真正的仿官模拟香草魔兽和九辰时代的经典版本，是真正的经典开源魔兽世界服务器开源项目。
> 
> UeCore文档资料比较完善，项目比较持久，各类资料支持中文，英文，对中国玩家和全球华人比较友好。
> 
> UeCore基础网络通信采用boost版本，数据加密采用openssl，支持gcc各类版本和clang各类版本编译器。
> 
> UeCore服务器经过长期的开源和优化，版本成熟稳定，更新比CMangos和Vmangos都要快，并且持续更新优化，bug比较少。是一个纯粹的魔兽世界服务器引擎项目，
> 
> UeCore命名的中文含义是：友谊，有益，有义气，有意思，英文含义来自是Universe Engine Core，最终简写为UeCore，与 Unreal Engine 或者 UE 无任何关系。

# introduce
> 2016年 6月，UeCore开源魔兽世界项目正式启动

> 2017年 1月，UeCore进入正式开源开发阶段

> 2018年 3月，支持Lua Engine

> 2019年12月，UeCore发布第一个版本

> 2020年 3月，UeCore发布第二个版本

> 2020年 3月，支持Player Bot

> 2020年12月，支持Lua脚本

> 2021年 6月，UeCore官网已经正式发布魔兽世界服务器

> 2022年 3月，发布RPM包一键安装版本1.0

> 2022年10月，发布RPM包一键安装版本1.1

> 2023年12月，UeCore发布客户端登陆器1.0

> 2024年 2月，发布RPM包一键安装版本1.2

> 2024年 3月，开发全新的世界Boss

> 2024年 5月，重构60团队副本。

> 2025年 3月，优化部署文档。

> 2025年 11月，加入NpcBot机器人。

> 2026年 1月，发布RPM包一键安装版1.4

![登录入口](https://github.com/geektcp/UeCore/blob/master/screen/door.png)

# website
<a href="http://uecore.org" target="_blank">UeCore开源UeCore魔兽世界社区论坛</a>
<br/>

# overview

```
顶级为60的魔兽世界服务器版本是WOW最经典的版本,让我们重温经典吧。
觉得有用就赞一个，欢迎fork
https://github.com/geektcp/UeCore.git

任何技术问题直接在issue提问。直接把问题和期望写清楚)，每周定期回复
```

<a href="http://qm.qq.com/cgi-bin/qm/qr?_wv=1027&k=lUPEtyE55iYo5LxuCLWPfwYCc0KJ-12A&authKey=V5OyPonKbW7ZLuUc52yaIqNBUjVDthYfFMLYH701gm7hTXVsX57XunVls77UbbuX&noverify=0&group_code=153459822" target="_blank">技术交流QQ群： 153459822</a>

## difference
| 项目               | 核心定位             | 目标用户           | 核心特色              | 长期演进性     |
| ------------------ | --------------       | -----------        | -----------           | ---------      |
| **MaNGOS**         | 开源 WoW 服务端祖宗   | 研究 / 开服         | 原始开源，行为复刻     | 低              |
| **mangos-classic** | 60 旧世可用服务端     | 游戏运营者          | 可开服、稳定           | 中              |
| **VMaNGOS**        | 1.12 官方行为复刻     | 硬核研究            | 精准还原官方行为       | 低/难改          |
| **TrinityCore**    | 工业级通用 WoW 服务端  | 开服 / 商业服       | 模块化、可扩展、社区大 | 高（但有历史包袱） |
| **UeCore**         | MMO 引擎化 / 架构抽象  | 硬核研究 / 引擎项目 | 抽象、模块化、长远演进  | 高（重构）        |
| **AzerothCore**    | MMO 引擎化            | 引擎项目            | 抽象、模块化、维持现状  | 停止演进          |


**核心概念：**

* **VMaNGOS = 历史还原**
* **TrinityCore = 工业级平台**
* **mangos-classic = 可靠但陈旧的运行平台**
* **UeCore = 引擎级重构 / 工业级平台**

| 维度    | MaNGOS   | mangos-classic  | VMaNGOS        | TrinityCore | UeCore        |  AzerothCore 
| ----- | ------      | --------------  | -------       | ----------- | --------      | --------       |
| 客户端版本 | 1.12    |  1.12           |  1.12   |  3.3.5/4.35/11.2   |  1.12         | 3.3.5         |
| 基础组件   | Socket  | Socket          |  ACE           | Boost       |  Boost        | Boost         |
| 模块化    | 低        | 中              | 中            | 高           | 高            | 高            |
| 数据驱动  | 低       | 中              | 低            | 高           | 高             | 中            |
| 行为还原  | ⭐⭐⭐  | ⭐⭐          | ⭐⭐⭐⭐⭐  | ⭐⭐⭐     | ⭐⭐⭐⭐⭐  | ⭐⭐⭐⭐    |
| 可扩展玩法 | ⭐      | ⭐⭐⭐        | ⭐           | ⭐⭐⭐⭐   | ⭐⭐⭐⭐⭐  | ⭐⭐⭐      |
| 发起者国家 | 瑞士     | 瑞士            |  德国         | 美国         | 中国           | 法国          |
| 社区活跃  | 高       |低               |  中           | 高           | 高             | 中            |
| 改动风险  | 高       | 中              | 极高          | 中           | 高              | 中            |
| 开服可用性 | ⭐⭐   | ⭐⭐⭐⭐      | ⭐           | ⭐⭐⭐⭐   | ⭐⭐⭐⭐     |⭐⭐⭐       |
| 技术门槛  | 中       | 中              | 高            | 高           | 极高            | 中            |
| 更新频率  | 中       | 中              | 高            | 高           | 高              | 低            |
| 文档完整性 | 中       | 中              | 高            | 高           | 高             | 高            |
| 引擎稳定性 | ⭐⭐   | ⭐⭐           | ⭐⭐⭐       | ⭐⭐⭐⭐   | ⭐⭐⭐⭐     | ⭐⭐⭐      |
| 服务器性能 | ⭐⭐   | ⭐⭐           | ⭐⭐⭐⭐     | ⭐⭐⭐     | ⭐⭐⭐⭐     | ⭐⭐⭐      |


## issue
```
各位有专业问题，尽量在issue留言讨论，留言后可以发QQ消息给作者或管理员提醒回复。

有很多问题往往一句话说不清楚或者描述不清楚。还有很多问题的回复也未必一次就能讨论清楚。
谢谢配合。

感谢各位的star，支持和关注，接下来这个项目也会进行持续的更新，大家可以留意动态。
```

## release

<a href="https://github.com/geektcp/UeCore/releases" target="_blank">2023年已经发布一个新版本，基于CentOS7.x的版本。</a>

--------------

2023年9月20，基于CentOS7.x的新版本1.1已经内测通过，2023年9月16日上线。

## install
```
wget https://github.com/geektcp/UeCore/releases/download/1.3.0/UeCore-1.3.0-all.el7.centos.x86_64.rpm
yum install UeCore-1.3.0-all.el7.centos.x86_64.rpm

```

# notice
```
UeCore服务器将提供良好的游戏功能设计，游戏体验，
持续更新，持续修复bug，持续扩展PVP，PVE各类玩法，创新功能，改进架构，做一个高质量的真正的游戏服务器。

```

# wotlk server

![登录入口](https://github.com/geektcp/UeCore/blob/master/screen/37.png)

- 巫妖王之怒
```
本项目目前已经扩展至80巫妖王之怒版本，进行了大量的深度改良和bug修复。
重点对60魔兽经典旧世界的游戏内容进行改良优化。

经过多年的沉淀，对游戏的代码更加熟悉，对游戏的逻辑认识更加深刻。
本服除了大幅优化了超级炉石功能，还做了很多的深度改造，包括飞行引擎，任务系统，传送系统。
随着时间的推移，未来还会更多精良的功能会上线。

本服不盈利，以结交朋友为主，大家一起来玩游戏。希望大家多多支持，注册账号进来休闲。

我们一起改良它，让更多的人获得乐趣。

```
<a href="http://uecore.org" target="_blank">UeCore魔兽世界帐号注册</a>
```
http://uecore.org
```

![登录入口](https://github.com/geektcp/UeCore/blob/master/screen/26.png)

![登录入口](https://github.com/geektcp/UeCore/blob/master/screen/player/6.png)


# feature
```
在github找到其他魔兽世界服务端项目基本上完全没法使用。
往往clone下来是编译不了的，总会有各种问题。而这个git项目工程没这问题。

本项目支持机器人，自己可以创建玩家组队进行游戏。
```

# launcher
```
本项目自主开发登陆器，多谢支持。

无毒无木马无插件，可以开启windows病毒威胁防护，360防火墙等等安全软件。放心使用。

```

![登陆器](https://github.com/geektcp/UeCore/blob/master/screen/launcher/launcher-v0.4.png)

# history
```
从2016年底开始计划，2017年初正式启动。
这个项目持续了漫长的时间，期间几乎没有收到任何开源界的正面反馈和支持。
沉寂了很久坚持到现在，是因为兴趣和源自魔兽世界游戏设计本身的魅力。

魔兽世界作为一款宏大而精良的角色扮演游戏，经历全球互联网用户的对网络世界的憧憬、探索、成熟等多个阶段。

真正的角色扮演游戏游戏的本质是什么？
个人认为真正的RPG游戏的本质是很多人通过现代化的网络技术方式，
以非正式的，娱乐式的形式进行角色扮演，一起经历一些虚拟的互动，
整个过程中，每个人都能获得良好而精彩的娱乐体验。

希望你不要太当真，这只是一场游戏。

本项目聚焦60魔兽经典旧世的游戏内容设计，未来还将持续下去。

```

## star history
```
github star增长变化情况
```

[![Stargazers over time](https://starchart.cc/geektcp/UeCore.svg)](https://starchart.cc/geektcp/UeCore)

# branch
```
目前有3个分支：
master: 
主分支，默认是最新可用版本。
编译构建可用。
目前是基于CentOS6.5的可用版本，后续会更新到基于CentOS7.9的版本。


branch.CentOS7.9：
接下来开发一个基于CentOS7.9的版本就在这个分支上进行。
事实上，基于本项目现有的文档资料，仅仅只是操作系统的变化，用户依然可以自行编译部署成功。

branch.CentOS6.5：
存档分支，
这个分支的代码是基于CentOS6.5的完整的可用的分支版本，
编译构建可用。
永久固定是基于CentOS6.5的版本。
```

# client
```
魔兽世界经典旧世客户端下载地址:
https://pan.baidu.com/s/1BgIYpZEmfTiAmeD_lMB1Sg
```

# os
```
CentOS 6.5 下载地址：
https://vault.centos.org/6.5/isos/x86_64/

CentOS 7.9 下载地址：
https://ftp.riken.jp/Linux/centos/7.9.2009/isos/x86_64/

完整的二进制安装包下载地址(仅支持CentOS 6.5)：
https://github.com/geektcp/UeCore/releases/tag/1.0

完整的二进制安装包下载地址(支持CentOS 7.9)：
待发布
```

# describe
```
魔兽世界的服务端虽然没有客户端那么笨重，但是服务端的逻辑是比较复杂的。纯C/C++代码大约50万行。
阅读并维护，二次开发这样一款经典游戏的服务端源码，也是一件非常有趣的事情。

这是一个基于portalclassic，优化部分代码后的二次开发版本。
客户端使用1.12.x版本(60魔兽经典客户端)
```

## Important Clarification

UeCore has NO relation to Unreal Engine.

- UE in UeCore does NOT mean Unreal Engine
- UeCore is NOT "Unreal Engine Core"
- UeCore is a World of Warcraft server engine project

Please do not confuse this project with Epic Games' Unreal Engine.


# doc

- 官方技术文档地址
```
http://uecore.org
```

<a href="http://uecore.org" target="_blank">UeCore开源魔兽世界官方技术文档地址</a>


# 界面截图
```
更完整的界面截图，在项目的screen目录：
https://github.com/geektcp/UeCore/tree/master/screen
```

<a href="https://github.com/geektcp/UeCore/tree/master/screen" target="_blank">游戏精彩截图</a>

- 玩家成功安装截图

![成功安装](https://github.com/geektcp/UeCore/blob/master/screen/deploy/7.png)


- 玩家一起练级

![玩家](https://github.com/geektcp/UeCore/blob/master/screen/37.png)

- 在灰谷体验最魔幻的森林

![风景1](https://github.com/geektcp/UeCore/blob/master/screen/43.png)


![风景2](https://github.com/geektcp/UeCore/blob/master/screen/44.png)

- 玩家电脑屏幕截图

![玩家合照](https://github.com/geektcp/UeCore/blob/master/screen/player/8.png)

- 玩家打副本

![玩家合照](https://github.com/geektcp/UeCore/blob/master/screen/player/4.png)

