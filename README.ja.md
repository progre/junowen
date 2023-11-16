# Ju.N.Owen

東方獣王園の非公式オンライン対戦ツールです。

非公式のツールです。**自己責任で使用してください。**

公式のオンライン対戦のマッチングや同期機構とは異なる、独自の仕組みでオンライン対戦を実現します。
adonis や th075caster と同じような仕組みで動作します。


## 特徴

* 公式のオンライン対戦よりもずれにくい
* ゲーム中にディレイを変更できる
* サーバーなしでも接続できる
* 観戦ができる


## インストール方法

1. zip ファイルを展開し、d3d9.dll と、modules フォルダーの中に th19_junowen.dll があることを確認します。
2. 獣王園のインストールフォルダーを開きます。
3. 獣王園のインストールフォルダーに d3d9.dll と modules フォルダーを移動します。
4. 獣王園を起動します。
5. うまくいけば獣王園のタイトル画面の項目に「Ju.N.Owen」が追加されます。


## 使い方

現在2つの接続方法をサポートしています。

### Shared Room (共用ルーム)

設定したルーム名と一致するユーザーと接続する方式です。

1. 「Online VS Mode」でルーム名を設定します。
2. 「Ju.N.Owen」→「Shared Room」を選択します。
3. 「Enter」の状態でショットボタンを押すと、接続の待ち受けが開始されます。
    * 「Leave」の状態でショットボタンを押すと、接続待ちが中断されます。
    * キャンセルボタンを押すと接続待ちのまま他の機能を使用できます。

### Pure P2P (サーバーを介さない接続)

接続サーバーを使わず、チャットなどで対戦相手と接続情報を交換する方式です。

1. 「Ju.N.Owen」→「Pure P2P」を選択します。
2. ホストとして接続を待ち受ける場合は「Connect as Host」を、
   ゲストとして接続する場合は「Connect as Guset」を選択します。
    * ホスト
        1. `<offer>********</offer>` という長い文字列が表示され、自動的にクリップボードにコピーされるので、
           この文字列を Discord 等を使って対戦相手に送信してください。
           「Copy your code」を選択すると再度クリップボードにコピーされます。
        2. 対戦相手から `<answer>********</answer>` という文字列を受け取り、
           クリップボードにコピーしてください。
        3. 「Paste guest's code」を選択してください。
        4. うまくいけば難易度選択に遷移し、対戦が開始されます。
    * ゲスト
        1. 対戦相手から `<offer>********</offer>` という文字列を受け取り、クリップボードにコピーしてください。
        2. ショットボタンを押すと、クリップボードの内容が入力されます。
        3. `<answer>********</answer>` という長い文字列が表示され、自動的にクリップボードにコピーされるので、
           この文字列を Discord 等を使って対戦相手に送信してください。
           ショットボタンを押すと再度クリップボードにコピーされます。
        4. うまくいけば難易度選択に遷移し、対戦が開始されます。

### 接続後

* 接続中はお互いの名前が画面上部に表示されます。切断されると表示が消えます。
* ホストはゲーム中に数字キーの0-9でディレイ値を変更できます。

### 観戦機能

現在は Pure P2P でのみ観戦が可能です。

* 観戦者
    1. 「Ju.N.Owen」→「Pure P2P」→「Connect as Spectator」を選択します。
    2. `<s-offer>********</s-offer>` という長い文字列が表示され、自動的にクリップボードにコピーされるので、
       この文字列を Discord 等を使ってプレイヤーのどちらかに送信してください。
       「Copy your code」を選択すると再度クリップボードにコピーされます。
    3. プレイヤーから `<s-answer>********</s-answer>` という文字列を受け取り、
       クリップボードにコピーしてください。
    4. 「Paste guest's code」を選択してください。
    5. うまくいけば観戦が開始されます。
    6. ポーズボタンを押すと観戦を中止します。
* プレイヤー
    1. Ju.N.Owen の対戦機能で対戦相手と接続し、難易度選択で待機します。
    2. 観戦者から `<s-offer>********</s-offer>` という文字列を受け取り、クリップボードにコピーしてください。
    3. F1 キーを押すと、クリップボードの内容が入力されます。
    4. `<answer>********</answer>` という長い文字列が表示され、自動的にクリップボードにコピーされるので、
       この文字列を Discord 等を使って対戦相手に送信してください。
    5. うまくいけば観戦が開始されます。


## 補足

* ポート開放は必要ありません。開放してあってもそのポートを指定することはできません。


## 現在の制約

* 「Online VS Mode」が解放されていないと正しく動作しません。
* 観戦者の追加はプレイヤーの接続直後のみ可能です。
* 観戦者の受け入れはプレイヤー一人につき1人のみです。
* 通信が遅延したり良くないことが起きるとゲームがフレーズすることがあります。


## 作者と配布元

[ぷろぐれ](https://bsky.app/profile/progre.me)

https://github.com/progre/junowen