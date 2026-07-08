#!/system/bin/sh
export PATH=/sbin:/system/sbin:/system/bin:/system/xbin
INPUT=/system/bin/input
AM=/system/bin/am

sleep $(( RANDOM % 181 ))
$INPUT keyevent 26
sleep 1
$INPUT swipe 540 2000 540 400 1000
sleep 3
$AM start -n com.midea.connect/com.meicloud.start.activity.SplashActivity
sleep 5
$INPUT tap 140 2200
sleep 2
$INPUT tap 672 2200
sleep 2
$INPUT swipe 540 2000 540 400 1000
sleep 1
$INPUT tap 537 1385
sleep 3
$INPUT keyevent 3
sleep 0.5
$INPUT keyevent 26
