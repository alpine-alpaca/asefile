

scaling = chop off
```
======> Writing cel: x:3..8, y:4..11
**** src=Rgba([0, 52685, 63993, 65535]),
 pixel=Rgba([60909, 30326, 5140, 32896]), a=255, opacity=128, new=Rgba([30573, 41461, 34451, 65534])
Pixel difference in tests\data\transparency_01.actual.png: 5,8
  expected: Rgba([118, 162, 135, 255])
    actual: Rgba([119, 161, 134, 255])
```


scaling = proportional
```
======> Writing cel: x:3..8, y:4..11
**** src=Rgba([0, 52685, 63993, 65535]), pixel=Rgba([60909, 30326, 5140, 32896]), a=255, opacity=128, new=Rgba([30573, 41461, 34451, 65534])
Pixel difference in tests\data\transparency_01.actual.png: 5,8
  expected: Rgba([118, 162, 135, 255])
    actual: Rgba([119, 161, 134, 255])

```


copied blend function
```
**** src=Rgba([0, 205, 249, 255]),
   pixel=Rgba([237, 118, 20, 255]), opacity=128,
     new=Rgba([119, 161, 134, 255])

expected=Rgba([118, 162, 135, 255])
```