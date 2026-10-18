from pathlib import Path
from html import escape

ROOT=Path(__file__).parent
BG='#FAF7F2'; INK='#272335'; MUTED='#655D70'; ACC='#7354A3'
def rect(x,y,w,h,fill='#FFFFFF',r=18,stroke='none'):
    return f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{r}" fill="{fill}" stroke="{stroke}"/>'
def text(x,y,s,size=16,fill=INK,weight=400):
    return f'<text x="{x}" y="{y}" font-size="{size}" fill="{fill}" font-weight="{weight}">{escape(s)}</text>'
def button(x,y,w,s,primary=True):
    return rect(x,y,w,48,ACC if primary else '#EEE8F4',14)+text(x+16,y+30,s,15,'#FFFFFF' if primary else ACC,600)
def badge(x,y,s,kind='mint'):
    fill,fg=('#E2EFE8','#285343') if kind=='mint' else ('#F8E9C9','#6A4913')
    return rect(x,y,len(s)*7.4+24,30,fill,15)+text(x+12,y+20,s,13,fg,600)
def art(x,y,w,h,kind=0):
    palettes=[('#E8E1F3','#A89BC5'),('#DBEAE5','#92B8A6'),('#F5E4D2','#DCB98B')]
    a,b=palettes[kind%3]
    return rect(x,y,w,h,a,18)+f'<circle cx="{x+w*.72}" cy="{y+h*.3}" r="{h*.16}" fill="{b}"/>'+f'<path d="M{x+12} {y+h-12} L{x+w*.35} {y+h*.35} L{x+w*.56} {y+h*.7} L{x+w*.72} {y+h*.45} L{x+w-12} {y+h-12} Z" fill="{b}"/>'
def shell(title,subtitle):
    s=rect(0,0,1510,960,BG,0)+text(32,40,'DOKI / DESIGN CONCEPT 01',15,ACC,700)+text(32,76,title,28,INK,700)+text(32,104,subtitle,15,MUTED)
    s+=rect(28,132,990,746,'#EEE9E3',24)+rect(38,142,970,726,BG,18)
    s+=rect(1052,132,430,746,'#302A3B',30)+rect(1064,144,406,722,BG,23)
    s+=text(60,183,'◉ doki',23,ACC,700)+text(60,229,'Library',16,ACC,700)+text(60,268,'Incoming  1',16)+text(60,307,'Devices',16)+text(60,813,'Settings',15,MUTED)
    s+=text(1084,182,'doki',22,ACC,700)+text(1350,181,'Practice',13,MUTED)
    s+=rect(1076,798,382,54,'#EEE8F4',15)+text(1090,831,'Library',15,ACC,600)+text(1210,831,'Incoming · 1',15,ACC,600)+text(1352,831,'Devices',15,ACC,600)
    s+=text(32,909,'PROPOSED • sample data • static mockup • no production functionality',16,ACC,700)
    s+=text(32,936,'Desktop concept + compact 406dp concept. Real device support and owner acceptance remain pending.',14,MUTED)
    return s
def save(name,s):
    (ROOT/name).write_text(f'<svg xmlns="http://www.w3.org/2000/svg" width="1510" height="960" viewBox="0 0 1510 960" role="img" aria-labelledby="title"><title id="title">Doki proposed {escape(name)} static concept with sample data</title><g font-family="Segoe UI,Arial,sans-serif">{s}</g></svg>',encoding='utf-8')

s=shell('Your progress, tucked away.','Library / calm status, artwork fallbacks, and explicit next actions')
s+=text(248,192,'Good evening, player',29,INK,700)+text(248,221,'3 games · checkpoints stay local until shared',16,MUTED)+button(818,175,154,'+ Connect game',False)
s+=rect(244,247,728,80,'#EEE8F4')+text(265,279,'✦ A new save is ready to review',18,INK,600)+text(265,305,'Your playable save has not changed.',15,MUTED)+button(819,262,132,'Review save')
for i,(name,state) in enumerate([('Moonlit Meadow','✓ Protected locally'),('Cloud Courier','! Needs your choice'),('Little Orbit','✓ Protected locally')]):
    x=244+i*246
    s+=rect(x,350,234,340)+art(x+12,362,210,157,i)+text(x+16,549,name,19,INK,700)+text(x+16,577,'Sample game · handheld',14,MUTED)+badge(x+12,595,state,'warn' if i==1 else 'mint')+text(x+16,659,'Last checkpoint: 8:14 PM',14,MUTED)
s+=text(1084,230,'Your library',26,INK,700)+text(1084,258,'Your playable saves stay in your control.',14,MUTED)
s+=rect(1082,278,370,88,'#EEE8F4')+text(1098,307,'✦ New save from your PC',17,INK,600)+text(1098,333,'Review before using it',14,MUTED)
s+=art(1082,387,370,146)+text(1094,562,'Moonlit Meadow',22,INK,700)+badge(1094,580,'✓ Protected locally')+text(1094,638,'Checkpoint · Oct 17, 8:14 PM',15,MUTED)+button(1094,662,340,'Create checkpoint')
save('01-library.svg',s)

s=shell('You played on both devices.','Incoming save review / preserve both continuations, make one deliberate choice')
s+=text(244,191,'Moonlit Meadow',28,INK,700)+text(244,221,'Both saves continue from checkpoint “Before the hill”.',16,MUTED)
for x,head,time,tag in [(244,'This handheld','Oct 17 · 8:14 PM','Current playable save'),(615,'Your PC','Oct 17 · 8:32 PM','Incoming checkpoint')]:
    s+=rect(x,250,356,265)+badge(x+18,268,tag,'warn' if x==615 else 'mint')+text(x+18,330,head,23,INK,700)+text(x+18,363,time,16,MUTED)+text(x+18,395,'Save bundle · 2 files · 128 KB',15,MUTED)+text(x+18,429,'Copy verified',15,'#285343',600)+text(x+18,468,'Progress details unavailable',14,MUTED)
s+=rect(244,535,727,116,'#EEE8F4')+text(264,565,'What happens next?',18,INK,700)+text(264,593,'Doki backs up this handheld’s save before replacement.',16)+text(264,621,'Both versions will stay in your history.',16)
s+=button(244,681,320,'Back up and use PC save')+button(580,681,215,'Keep handheld save',False)+text(244,762,'No automatic merge: these binary saves need a chosen continuation.',14,MUTED)
s+=text(1084,229,'Choose a continuation',23,INK,700)+text(1084,257,'Moonlit Meadow · sample saves',14,MUTED)
for y,head,stamp,current in [(280,'This handheld','Oct 17 · 8:14 PM',True),(440,'Your PC','Oct 17 · 8:32 PM',False)]:
    s+=rect(1082,y,370,144)+text(1098,y+32,head,20,INK,700)+badge(1098,y+45,'Current playable' if current else 'Incoming checkpoint','mint' if current else 'warn')+text(1098,y+104,stamp+' · Copy verified',14,MUTED)
s+=text(1090,617,'Both saves stay in history.',16,INK,600)+text(1090,644,'Your current save is backed up first.',14,MUTED)+button(1084,667,366,'Back up and use PC save')+button(1084,727,366,'Keep handheld save',False)
save('02-review.svg',s)

s=shell('A little history. A way back.','Checkpoint history / clear identity, restore review, and backup before replacement')
s+=text(244,192,'Moonlit Meadow / History',27,INK,700)+text(244,223,'3 checkpoints · sample history',16,MUTED)
for y,name,device in [(260,'Evening checkpoint','Handheld · Oct 17, 8:14 PM'),(382,'Before the hill','PC · Oct 17, 6:20 PM'),(504,'First steps','Handheld · Oct 16, 9:02 PM')]:
    s+=rect(244,y,340,104)+text(264,y+32,name,18,INK,700)+text(264,y+63,device,14,MUTED)+text(264,y+87,'✓ Copy verified',13,'#285343')
s+=rect(608,258,364,412)+text(630,295,'Restore “Before the hill”?',22,INK,700)+text(630,329,'PC · Oct 17, 6:20 PM',15,MUTED)+text(630,365,'Replaces the playable save on',16)+text(630,390,'this handheld after a verified backup.',16)+text(630,432,'Your newer progress stays in history.',15,MUTED)+text(630,467,'Game compatibility: matching profile',14,MUTED)+button(630,505,316,'Back up and restore')+button(630,566,316,'Cancel',False)
s+=text(1084,229,'Checkpoint history',25,INK,700)+text(1084,258,'Moonlit Meadow',15,MUTED)+rect(1082,283,370,112)+text(1098,315,'Before the hill',20,INK,700)+text(1098,345,'PC · Oct 17, 6:20 PM',15,MUTED)+badge(1098,354,'✓ Copy verified')
s+=rect(1082,415,370,342,'#EEE8F4')+text(1100,450,'Restore this checkpoint?',23,INK,700)+text(1100,486,'Doki backs up the current save',16)+text(1100,512,'before replacing it on this handheld.',16)+text(1100,551,'Newer progress stays in history.',15,MUTED)+button(1100,580,334,'Back up and restore')+button(1100,642,334,'Cancel',False)+text(1100,726,'Proposed confirmation sheet',13,MUTED)
save('03-history.svg',s)

s=rect(0,0,1510,960,BG,0)+text(40,53,'Doki / proposed component and token gallery',30,INK,700)+text(40,86,'Sample data • static design preparation • no approved mockups or production behavior',16,MUTED)
for i,(label,color) in enumerate([('Canvas',BG),('Ink',INK),('Muted',MUTED),('Accent',ACC),('Mint','#E2EFE8'),('Attention','#F8E9C9')]):
    x=40+i*240;s+=rect(x,120,212,80,color,18, '#D7CEDD')+text(x,228,label+' '+color,15)
s+=text(40,280,'Type / Segoe UI or native system sans',24,INK,700)+text(40,322,'Heading · 28/36 · semibold',28,INK,600)+text(40,359,'Body · 16/24 · Keep your progress, on your terms.',16)+text(40,391,'Metadata · 14/20 · Device + absolute capture time',14,MUTED)
s+=button(40,430,240,'Create checkpoint')+button(300,430,210,'Review details',False)+rect(530,426,246,56,'none',18,ACC)+button(534,430,238,'Keyboard focus')
s+=rect(800,430,235,48,'#E5E0E7',14)+text(816,460,'Unavailable · disabled',15,MUTED)+badge(40,510,'✓ Protected locally')+badge(260,510,'! Needs your choice','warn')
s+=rect(40,570,650,118)+text(60,604,'! Folder access needs updating',20,INK,700)+text(60,634,'Your checkpoints remain in the vault.',16,MUTED)+text(60,665,'Reconnect folder  →',16,ACC,600)
s+=rect(730,570,730,118)+text(750,603,'Creating checkpoint · verifying copied bytes',18,INK,600)+rect(750,626,680,12,'#EEE8F4',6)+rect(750,626,390,12,ACC,6)+text(750,669,'Illustrated measurable progress; no fake percentages.',14,MUTED)
s+=text(40,744,'Spacing: 4 / 8 / 12 / 16 / 24 / 32     Radius: 14 controls / 18 cards / 24 panels',18,INK,600)
s+=text(40,785,'48dp targets • visible focus • text + icon status • wrapping layouts at 200% text',17)
s+=text(40,823,'Motion proposal: 160ms control / 220ms panel. Reduced motion: no translation or bounce.',17)
s+=text(40,864,'Screen readers: named actions + device/time/state; announce final results once. Test, do not assume compliance.',16,MUTED)
s+=text(40,918,'Asset provenance: original SVG shapes and system text; no Cocoon artwork, screenshots, or game assets copied.',15,ACC)
save('04-components.svg',s)

links=''.join(f'<section><h2>{label}</h2><img src="{file}" alt="{label}: proposed desktop and compact layouts with sample data"></section>' for file,label in [('01-library.svg','Library'),('02-review.svg','Incoming save review'),('03-history.svg','History and restore'),('04-components.svg','Components and tokens')])
(ROOT/'index.html').write_text('<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Doki D01 proposed concepts</title><style>body{font:16px/1.5 system-ui;background:#FAF7F2;color:#272335;margin:24px}main{max-width:1510px;margin:auto}img{width:100%;height:auto;border:1px solid #d7cedd;border-radius:16px}h1{color:#7354A3}section{margin:32px 0}a{color:#7354A3}:focus-visible{outline:3px solid #7354A3;outline-offset:4px}</style><main><h1>Doki · proposed design concepts</h1><p>Original static mockups for D01 #31. Sample data. No production functionality. Owner acceptance pending. These are design preparation; F01 remains the sole engineering story.</p><p>SVG sheets are scalable visual concepts, not accessible application implementations. Read <a href="README.md">design notes and limitations</a>. No animations run.</p>'+links+'</main></html>',encoding='utf-8')
